//! Renderer abstraction. Real implementation uses Smithay's multi renderer
//! (GLES2 + pixman fallback). For the first ISO we just clear with the
//! configured background colour via a stub.

pub enum Renderer {
    None,
    Stub { cleared: bool },
}

impl Renderer {
    pub fn new() -> Result<Self, &'static str> {
        // TODO: try GLES2/GBM via Smithay (behind `real-gpu` feature); fall back to Stub.
        Ok(Self::Stub { cleared: false })
    }
    pub fn clear_background(&mut self, rgb: &[u8; 3]) {
        if let Renderer::Stub { cleared } = self {
            if !*cleared {
                eprintln!(
                    "[novai-comp] cleared background to #{:02x}{:02x}{:02x}",
                    rgb[0], rgb[1], rgb[2]
                );
                *cleared = true;
            }
        }
    }
}
