# Dynamic Island — Fedora system package (RPM spec)
#
# Build with: rpmbuild --define "_sourcedir $PWD" --define "_specdir $PWD/packaging" --define "_builddir $PWD" --define "_srcrpmdir $PWD" --define "_rpmdir $PWD" -bb packaging/dynamic-island.spec
# Or: just package-rpm (requires rpmbuild, not present in this devshell).
#
# The system package installs to /usr/local + /lib/systemd/system (with
# sudo). The default, no-sudo path is the user-local tarball from
# `just package-fedora` (see packaging/fedora.yaml). This spec exists so
# the system path is buildable where rpmbuild is available, and so the
# file list / dependencies are reviewable.

Name:           dynamic-island
Version:        0.1.0
Release:        1%{?dist}
Summary:        Dynamic Island shell for Sway/niri/hyprland
License:        MIT
URL:            https://example.invalid/dynamic-island
Source0:        isle-fedora.tar.gz
BuildArch:      x86_64
Requires:       systemd-user
Requires:       quickshell >= 0.3.2
Requires:       mesa-libGL
Requires:       (awww or hyprpaper)
Requires:       xdg-utils

%description
Dynamic Island shell: a Rust daemon (isled) plus a Quickshell/QML
interface pinned to the topmost (overlay) layer. Panels, wallpapers,
notifications, tray, calendar, todos, notes, clipboard, and a context
engine (logind/UPower/NetworkManager/PipeWire) all speak over a
Unix-socket IPC.

%prep
%setup -n isle-fedora -q
# Source layout inside isle-fedora.tar.gz (from `just package-fedora`):
#   isle-fedora/bin/isled
#   isle-fedora/ui/**.qml
#   isle-fedora/systemd/*.template
#   isle-fedora/install.sh
#   isle-fedora/FEDORA.md
#   isle-fedora/sway-binds.conf

%install
rm -rf %{buildroot}
install -d %{buildroot}/usr/local/bin
install -d %{buildroot}/usr/local/share/quickshell/isle
install -d %{buildroot}/usr/lib/systemd/user
install -m 0755 bin/isled %{buildroot}/usr/local/bin/isled
cp -R ui/. %{buildroot}/usr/local/share/quickshell/isle/
# System units are rendered at %post (EGL env needs store paths); templates
# ship so %post can render them. User units remain the default install.
install -m 0644 systemd/isled.service.template %{buildroot}/usr/lib/systemd/user/isled.service
install -m 0644 systemd/quickshell.service.template %{buildroot}/usr/lib/systemd/user/quickshell.service
install -m 0644 FEDORA.md %{buildroot}/usr/local/share/quickshell/isle/FEDORA.md
install -m 0644 sway-binds.conf %{buildroot}/usr/local/share/quickshell/isle/sway-binds.conf

%post
# Render @PLACEHOLDER@ tokens for the system install. EGL lines are only
# needed on Fedora-with-Nix (see FEDORA.md); NixOS ignores them.
if [ -x /usr/local/bin/isled ]; then
  sed -e "s|@ISLED_BIN@|/usr/local/bin/isled|g" \
      -e "s|@QUICKSHELL_BIN@|/usr/local/bin/quickshell|g" \
      -e "s|@UI_PATH@|/usr/local/share/quickshell/isle|g" \
      /usr/lib/systemd/user/isled.service.template > /usr/lib/systemd/user/isled.service 2>/dev/null || true
fi
systemctl --user daemon-reload 2>/dev/null || true

%preun
systemctl --user disable --now isled quickshell 2>/dev/null || true

%files
/usr/local/bin/isled
/usr/local/share/quickshell/isle/
/usr/lib/systemd/user/isled.service
/usr/lib/systemd/user/quickshell.service

%changelog
* Tue Nov 10 2026 asuna <asuna@fedora.local> - 0.1.0-1
- Initial system RPM spec for Fedora 45 + Sway/niri/hyprland.
