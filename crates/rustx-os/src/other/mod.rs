//! Every platform that is none of the three.
//!
//! Windows, Linux and macOS each have a directory now; this is what is
//! left — a BSD, a Redox, whatever comes. Every capability refuses in a
//! sentence that names the platform, because a program that builds
//! everywhere and silently does nothing somewhere is worse than one
//! that says where it stands.
//!
//! One file rather than a directory, deliberately: there is nothing to
//! fill in per capability here, only the same refusal repeated, and
//! splitting it would invite somebody to fill one of the files for a
//! platform nobody has.

pub mod clipboard {
    use crate::clipboard::Image;
    use crate::error::OsError;

    const WHAT: &str = "the clipboard";

    pub fn text() -> Result<String, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn set_text(_text: &str) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn image() -> Result<Image, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn set_image(_image: &Image) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn custom(_format: &str) -> Result<Vec<u8>, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn set_custom(_format: &str, _bytes: &[u8]) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }

    pub const AVAILABLE: bool = false;
}

pub mod autostart {
    use crate::error::OsError;

    pub const AVAILABLE: bool = false;

    pub fn set(_on: bool, _name: &str) -> Result<(), OsError> {
        Err(OsError::Unsupported("le démarrage automatique"))
    }

    pub fn is_on(_name: &str) -> bool {
        false
    }
}

pub mod instance {
    use crate::error::OsError;

    const WHAT: &str = "l'instance unique";

    pub const AVAILABLE: bool = false;

    pub struct Only;

    impl Only {
        pub fn one(_name: &str) -> Result<Self, OsError> {
            Err(OsError::Unsupported(WHAT))
        }
    }

    pub fn hand_over(_name: &str, _arguments: &[String]) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }

    pub fn handed_over() -> Vec<String> {
        Vec::new()
    }
}

pub mod notify {
    use crate::error::OsError;
    use crate::notify::What;

    pub const AVAILABLE: bool = false;

    pub fn show(_what: &What) -> Result<(), OsError> {
        Err(OsError::Unsupported("les notifications"))
    }

    pub const CAN_ASK: bool = false;

    pub fn ask(
        _what: &What,
        _actions: &[crate::notify::Action],
    ) -> Result<Option<String>, OsError> {
        Err(OsError::Unsupported("les notifications"))
    }
}

pub mod shortcut {
    use crate::error::OsError;

    pub const AVAILABLE: bool = false;

    pub struct Shortcut;

    impl Shortcut {
        pub fn listen(_modifiers: u32, _key: u32, _keys: &str) -> Result<Self, OsError> {
            Err(OsError::Unsupported("les raccourcis globaux"))
        }

        pub fn pressed(&self) -> u64 {
            0
        }
    }
}

pub mod tray {
    use std::path::Path;

    use crate::error::OsError;
    use crate::notify::What;
    use crate::tray::Item;

    const WHAT: &str = "l'icône de barre système";

    pub const AVAILABLE: bool = false;

    pub fn shown() -> usize {
        0
    }

    pub struct Icon;

    impl Icon {
        pub fn new(_icon: &Path, _tooltip: &str) -> Result<Self, OsError> {
            Err(OsError::Unsupported(WHAT))
        }

        pub fn hidden() -> Result<Self, OsError> {
            Err(OsError::Unsupported(WHAT))
        }

        pub fn menu(&self, _items: &[Item]) -> Result<(), OsError> {
            Err(OsError::Unsupported(WHAT))
        }

        pub fn clicked(&self) -> Option<String> {
            None
        }

        pub fn notify(&self, _what: &What) -> Result<(), OsError> {
            Err(OsError::Unsupported(WHAT))
        }
    }
}

pub mod dialog {
    use std::path::PathBuf;

    use crate::dialog::{Ask, DialogError};

    const WHAT: &str = "le dialogue de fichiers";

    pub const AVAILABLE: bool = false;

    pub fn open(_ask: &Ask) -> Result<Vec<PathBuf>, DialogError> {
        Err(DialogError::Unsupported(WHAT))
    }

    pub fn save(_ask: &Ask) -> Result<PathBuf, DialogError> {
        Err(DialogError::Unsupported(WHAT))
    }

    pub fn folder(_ask: &Ask) -> Result<PathBuf, DialogError> {
        Err(DialogError::Unsupported(WHAT))
    }
}

pub mod devices {
    use crate::devices::{Device, Kind};

    pub fn explain() {
        println!(
            "the device layer is Windows for now; this is {}",
            std::env::consts::OS
        );
    }

    pub fn devices_of(_kind: Kind) -> Vec<Device> {
        Vec::new()
    }
}

pub mod window {
    //! La fenêtre, partout ailleurs.
    //!
    //! Ni Windows, ni Linux, ni macOS : les trois ont leur dossier,
    //! et celui-ci est ce qui reste — un BSD, un Redox, ce qui
    //! viendra. Refuser en nommant la plateforme est tout ce qu'il y a
    //! à faire ici.

    use crate::error::OsError;
    use crate::window::{Dressing, Pane, Spread};

    pub const AVAILABLE: bool = false;

    const WHAT: &str = "window control";

    pub fn dress(_pane: Pane, _dressing: Dressing) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn spread(_pane: Pane, _how: Spread) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn spread_of(_pane: Pane) -> Result<Spread, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn on_top(_pane: Pane, _above: bool) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn drag_out(_pane: Pane, _paths: &[std::path::PathBuf]) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn drag(_pane: Pane) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn close(_pane: Pane) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn pointer() -> Result<(i32, i32), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}

pub mod system {
    use crate::error::OsError;
    use crate::system::{Power, Theme};

    const WHAT: &str = "the system state";

    pub fn theme() -> Result<Theme, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn power() -> Result<Power, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn idle() -> Result<std::time::Duration, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn locked() -> Result<bool, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn locale() -> Result<String, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn timezone() -> Result<String, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn scale() -> Result<f64, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}

pub mod shell {
    use std::path::Path;

    use crate::error::OsError;
    use crate::shell::Association;

    const WHAT: &str = "the file manager";

    pub fn reveal(_path: &Path) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn remember_recent(_path: &Path) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn associate(_association: &Association) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn dissociate(_association: &Association) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}

pub mod secret {
    use crate::error::OsError;

    const WHAT: &str = "secret storage";

    pub fn store(_service: &str, _account: &str, _secret: &str) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn fetch(_service: &str, _account: &str) -> Result<Option<String>, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn forget(_service: &str, _account: &str) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}

pub mod attention {
    use crate::error::OsError;
    use crate::window::Pane;

    const WHAT: &str = "the taskbar";

    pub fn flash(_pane: Pane) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn progress(_pane: Pane, _done: Option<f64>) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn badge(_count: Option<u32>) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}

pub mod color {
    use crate::color::Rgb;
    use crate::error::OsError;

    pub fn pick(_initial: Option<Rgb>) -> Result<Option<Rgb>, OsError> {
        Err(OsError::Unsupported("the color picker"))
    }
}

pub mod screen {
    use crate::error::OsError;
    use crate::screen::Screen;

    pub fn screens() -> Result<Vec<Screen>, OsError> {
        Err(OsError::Unsupported("the list of screens"))
    }
}

pub mod net {
    use crate::error::OsError;
    use crate::net::Network;

    pub fn network() -> Result<Network, OsError> {
        Err(OsError::Unsupported("the network state"))
    }
}

pub mod audio {
    use crate::audio::{Endpoint, Flow, Volume};
    use crate::error::OsError;

    const WHAT: &str = "the audio devices";

    pub fn endpoints(_flow: Flow) -> Result<Vec<Endpoint>, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn volume() -> Result<Volume, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn set_volume(_level: f64) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn set_muted(_muted: bool) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}

pub mod capture {
    use crate::clipboard::Image;
    use crate::error::OsError;
    use crate::screen::Area;
    use crate::window::Pane;

    const WHAT: &str = "capturing the screen";

    pub fn area(_area: Option<Area>) -> Result<Image, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn window(_pane: Pane) -> Result<Image, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}

pub mod bluetooth {
    use crate::bluetooth::Paired;
    use crate::error::OsError;

    pub fn paired() -> Result<Vec<Paired>, OsError> {
        Err(OsError::Unsupported("the Bluetooth devices"))
    }
}

pub mod about {
    use crate::about::Credits;
    use crate::error::OsError;
    use crate::window::Pane;

    pub const AVAILABLE: bool = false;

    pub fn show(_pane: Pane, _credits: &Credits) -> Result<(), OsError> {
        Err(OsError::Unsupported("the About panel"))
    }
}

pub mod print {
    use std::path::Path;

    use crate::error::OsError;
    use crate::print::Printer;

    const WHAT: &str = "printing";

    pub fn printers() -> Result<Vec<Printer>, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn print(_path: &Path, _ask: bool) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}

pub mod terminal {
    use std::path::Path;

    use crate::error::OsError;
    use crate::terminal::{Output, Shell};

    const WHAT: &str = "a terminal";

    pub fn shell() -> Result<Shell, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn run(_script: &str, _at: Option<&Path>) -> Result<Output, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
    pub fn open(_at: &Path) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}
