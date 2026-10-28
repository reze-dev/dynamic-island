#!/usr/bin/env python3
"""Fake input, for testing what the shell does with the keyboard.

The island is a layer surface with the keyboard on demand, so niri only
gives it the keyboard after a click. There is no way to test that from
inside the shell: the thing under test is the compositor's answer to a
pointer, and the shell cannot ask itself.

So this makes a pointer and a keyboard out of /dev/uinput and uses them.

    tools/fake-input.py click 700 155
    tools/fake-input.py type "git log"
    tools/fake-input.py key esc
    tools/fake-input.py move 700 155

The pointer is absolute, in physical pixels, so the coordinates are the
ones a screenshot is in.
"""

import ctypes
import ctypes.util
import fcntl
import os
import struct
import sys
import time

UI_SET_EVBIT = 0x40045564
UI_SET_KEYBIT = 0x40045565
UI_SET_RELBIT = 0x40045566
UI_SET_ABSBIT = 0x40045567
UI_SET_PROPBIT = 0x4004556B
UI_DEV_SETUP = 0x405C5503
UI_ABS_SETUP = 0x401C5504
UI_DEV_CREATE = 0x5501

EV_SYN = 0x00
EV_KEY = 0x01
EV_ABS = 0x03

INPUT_PROP_DIRECT = 0x01

SYN_REPORT = 0
ABS_X = 0x00
ABS_Y = 0x01

BTN_LEFT = 0x110
BTN_RIGHT = 0x111

# The keycodes the kernel uses, for the keys this can type.
KEYS = {
    "esc": 1, "escape": 1,
    "enter": 28, "return": 28,
    "tab": 15,
    "space": 57,
    "backspace": 14,
    "left": 105, "right": 106, "up": 103, "down": 108,
    "home": 102, "end": 107,
    "pageup": 104, "pagedown": 109,
    "ctrl": 29, "shift": 42, "alt": 56, "super": 125,
}

# The letters are not contiguous, so they get their own table.
for _letter, _code in zip(
    "abcdefghijklmnopqrstuvwxyz",
    [30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50, 49, 24, 25,
     16, 19, 31, 20, 22, 47, 17, 45, 21, 44],
):
    KEYS[_letter] = _code

for _digit in range(10):
    KEYS[str(_digit)] = 2 + (_digit % 10)


class InputId(ctypes.Structure):
    _fields_ = [
        ("bustype", ctypes.c_uint16),
        ("vendor", ctypes.c_uint16),
        ("product", ctypes.c_uint16),
        ("version", ctypes.c_uint16),
    ]


class UinputSetup(ctypes.Structure):
    """The kernel's struct uinput_setup."""

    _fields_ = [
        ("id", InputId),
        ("name", ctypes.c_char * 80),
        ("ff_effects_max", ctypes.c_uint32),
    ]


class InputAbsinfo(ctypes.Structure):
    _fields_ = [
        ("value", ctypes.c_int32),
        ("minimum", ctypes.c_int32),
        ("maximum", ctypes.c_int32),
        ("fuzz", ctypes.c_int32),
        ("flat", ctypes.c_int32),
        ("resolution", ctypes.c_uint32),
    ]


class UinputAbsSetup(ctypes.Structure):
    """The kernel's struct uinput_abs_setup: one axis' worth of range."""

    _fields_ = [
        ("code", ctypes.c_uint16),
        ("filler", ctypes.c_uint16),
        ("absinfo", InputAbsinfo),
    ]


class InputEvent(ctypes.Structure):
    _fields_ = [
        ("time", ctypes.c_long * 2),
        ("type", ctypes.c_uint16),
        ("code", ctypes.c_uint16),
        ("value", ctypes.c_int32),
    ]


class Input:
    def __init__(self, width=1920, height=1080):
        self.fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
        self.width = width
        self.height = height

        fcntl.ioctl(self.fd, UI_SET_EVBIT, EV_KEY)
        fcntl.ioctl(self.fd, UI_SET_EVBIT, EV_SYN)
        fcntl.ioctl(self.fd, UI_SET_EVBIT, EV_ABS)
        fcntl.ioctl(self.fd, UI_SET_ABSBIT, ABS_X)
        fcntl.ioctl(self.fd, UI_SET_ABSBIT, ABS_Y)
        # A direct pointer: the coordinates are the screen's, not deltas.
        fcntl.ioctl(self.fd, UI_SET_PROPBIT, INPUT_PROP_DIRECT)
        fcntl.ioctl(self.fd, UI_SET_KEYBIT, BTN_LEFT)
        fcntl.ioctl(self.fd, UI_SET_KEYBIT, BTN_RIGHT)
        for code in set(KEYS.values()):
            fcntl.ioctl(self.fd, UI_SET_KEYBIT, code)

        setup = UinputSetup()
        setup.id = InputId(0x03, 0x1234, 0x5678, 1)
        setup.name = b"isle-fake-input"
        fcntl.ioctl(self.fd, UI_DEV_SETUP, setup)

        # Tenths of a millimetre, the way an absolute pointer usually
        # describes itself, so the compositor can scale it to the output.
        for code, extent in ((ABS_X, width), (ABS_Y, height)):
            axis = UinputAbsSetup()
            axis.code = code
            axis.absinfo.minimum = 0
            axis.absinfo.maximum = extent * 10
            axis.absinfo.resolution = 10
            fcntl.ioctl(self.fd, UI_ABS_SETUP, axis)

        fcntl.ioctl(self.fd, UI_DEV_CREATE)

        # Give the compositor a moment to notice the device.
        time.sleep(0.3)

    def emit(self, etype, code, value):
        event = InputEvent()
        event.time = (int(time.time()), 0)
        event.type = etype
        event.code = code
        event.value = value
        os.write(self.fd, bytes(event))

    def sync(self):
        self.emit(EV_SYN, SYN_REPORT, 0)

    def move(self, x, y):
        self.emit(EV_ABS, ABS_X, int(x * 10))
        self.emit(EV_ABS, ABS_Y, int(y * 10))
        self.sync()

    def click(self, x, y, button=BTN_LEFT):
        self.move(x, y)
        time.sleep(0.05)
        self.emit(EV_KEY, button, 1)
        self.sync()
        time.sleep(0.05)
        self.emit(EV_KEY, button, 0)
        self.sync()
        time.sleep(0.05)

    def key(self, code):
        self.emit(EV_KEY, code, 1)
        self.sync()
        time.sleep(0.03)
        self.emit(EV_KEY, code, 0)
        self.sync()
        time.sleep(0.03)

    def type(self, text):
        for char in text:
            if char == " ":
                self.key(KEYS["space"])
                continue
            code = KEYS.get(char.lower())
            if code is None:
                raise SystemExit("cannot type %r" % char)
            self.key(code)

    def close(self):
        try:
            os.close(self.fd)
        except OSError:
            pass


def main(argv):
    if len(argv) < 2:
        raise SystemExit(__doc__)

    action = argv[1]
    device = Input()
    try:
        if action == "click":
            device.click(int(argv[2]), int(argv[3]))
        elif action == "move":
            device.move(int(argv[2]), int(argv[3]))
        elif action == "type":
            device.type(" ".join(argv[2:]))
        elif action == "key":
            for name in argv[2:]:
                code = KEYS.get(name.lower())
                if code is None:
                    raise SystemExit("no such key: %s" % name)
                device.key(code)
        else:
            raise SystemExit("no such action: %s" % action)
    finally:
        device.close()


if __name__ == "__main__":
    main(sys.argv)
