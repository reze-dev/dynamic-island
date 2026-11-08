pub mod compositor;
pub mod context;
pub mod live;
pub mod protocol;
pub mod state;

pub use compositor::{CompositorEvent, CompositorInfo, FocusedWindow, Output, Workspace};
pub use context::{Context, ContextState, Signal, is_quiet, looks_like_meeting, score, suggest};
pub use live::{Activity, ActivityKind, ActivityQueue, ActivityTone};
pub use protocol::{ClientMessage, ServerMessage};
pub use state::{Compositor, Features, PROTOCOL_VERSION, Session, Snapshot, SystemStats};
