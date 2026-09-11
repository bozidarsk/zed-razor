use zed_extension_api as zed;

struct Razor;

impl zed::Extension for Razor {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(Razor);
