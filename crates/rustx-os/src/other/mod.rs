//! Every platform that is neither Windows nor Linux.
//!
//! macOS above all, which will get a directory of its own the day
//! somebody writes it: `AppKit`'s `NSPasteboard` for the clipboard and
//! `AVFoundation` for the devices. Until then this crate compiles there
//! and refuses in a sentence that names the platform, because a program
//! that builds everywhere and silently does nothing on two of the three
//! is worse than one that says where it stands.

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
    //! macOS avant tout, où ce sera `NSWindow` : `styleMask` sans
    //! `titled` pour enlever le cadre, `performWindowDragWithEvent:`
    //! pour le déplacement, `toggleFullScreen:` pour l'écran entier —
    //! trois appels, le jour où ce dossier existera.

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
