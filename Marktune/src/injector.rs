use anyhow::Result;
use enigo::{Enigo, Keyboard, Settings};
#[cfg(target_os = "windows")]
use enigo::{Direction, Key};

/// لایه انتزاعی درج متن؛ برای پورت یا تعویض بک‌اند فقط همین trait را پیاده کن.
pub trait TextInjector {
    fn insert(&mut self, text: &str) -> Result<()>;
}

pub struct EnigoInjector {
    enigo: Enigo,
}

impl EnigoInjector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enigo: Enigo::new(&Settings::default())
                .map_err(|e| anyhow::anyhow!("init enigo: {e:?}"))?,
        })
    }

    /// روی ویندوز، اگر هنگام فراخوانی هات‌کی مودیفایرهایی مثل Ctrl/Alt/Shift
    /// هنوز از نظر فیزیکی پایین باشند، برخی اپ‌ها (مثل Illustrator) ورودی
    /// کاراکتری VK_PACKET را نادیده می‌گیرند چون آن را بخشی از یک shortcut
    /// در حال پردازش می‌بینند. برای دور زدن این حالت، یک keyup مصنوعی برای
    /// این مودیفایرها می‌فرستیم تا از نظر اپ مقصد، هیچ modifier‌ای پایین نباشد.
    #[cfg(target_os = "windows")]
    fn release_modifiers(&mut self) {
        for k in [Key::Control, Key::Alt, Key::Shift, Key::Meta] {
            let _ = self.enigo.key(k, Direction::Release);
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn release_modifiers(&mut self) {}
}

impl TextInjector for EnigoInjector {
    fn insert(&mut self, text: &str) -> Result<()> {
        self.release_modifiers();
        // روی مک از CGEventKeyboardSetUnicodeString استفاده می‌شود،
        // پس کاراکترهای غیرASCII مثل ٪ بدون دستکاری layout درج می‌شوند.
        self.enigo
            .text(text)
            .map_err(|e| anyhow::anyhow!("insert text: {e:?}"))
    }
}
