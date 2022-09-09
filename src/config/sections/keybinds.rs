use compositor_macros::config_section;

use crate::config::ConfigurationSection;

config_section!(
    Keybinds {
        "Move focused window to `d`th on the taskbar."
        window(d)       => (Meta+{d}),
    }
);

impl ConfigurationSection for Keybinds {
    const PATH : &'static str = "$.keybinds";
}
