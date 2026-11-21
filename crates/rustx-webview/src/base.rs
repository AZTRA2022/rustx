//! What every window starts with: Inter as the default font, and the colours
//! the default screens (system pages, not-found, error) are drawn in.
//!
//! The theme is Next.js's: black on white, white on black by the system's
//! preference, hairline borders, a black-or-white primary button. It is
//! written as CSS variables (`--rx-bg`, `--rx-fg`, `--rx-muted`,
//! `--rx-border`, `--rx-accent`), so an application's own pages may use them
//! and a `system/*.rsx` of its own looks the same as the defaults.
//!
//! Only `html` gets the font: a page that names its own on `body` or below
//! keeps it. Inter is the variable latin subset (SIL Open Font License, see
//! `assets/Inter-OFL.txt`), carried in the binary as a `data:` URI so no
//! request is made and the window works offline.

const INTER: &[u8] = include_bytes!("../assets/inter-latin-wght-normal.woff2");

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The stylesheet.
pub fn css() -> String {
    format!(
        "@font-face{{font-family:Inter;font-style:normal;font-weight:100 900;font-display:swap;\
         src:url(data:font/woff2;base64,{}) format('woff2')}}\
         :root{{--rx-bg:#fff;--rx-fg:#000;--rx-muted:#666;--rx-border:#eaeaea;--rx-accent:#0070f3;--rx-danger:#e5484d}}\
         @media (prefers-color-scheme:dark){{:root{{--rx-bg:#000;--rx-fg:#fff;--rx-muted:#a1a1a1;--rx-border:#333;--rx-accent:#3291ff;--rx-danger:#ff6369}}}}\
         html{{font-family:Inter,system-ui,\"Segoe UI\",Roboto,Helvetica,Arial,sans-serif;\
         -webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}}",
        base64(INTER)
    )
}

/// The script that puts it in every page, before the page's own.
pub fn script() -> String {
    format!(
        "(function(){{var put=function(){{var s=document.createElement('style');s.id='rustx-base';\
         s.textContent={};document.documentElement.appendChild(s)}};\
         if(document.documentElement)put();\
         else document.addEventListener('readystatechange',put,{{once:true}})}})();",
        serde_json::json!(css())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_is_the_standard_one() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn the_font_is_a_woff2_and_the_stylesheet_names_it() {
        assert_eq!(&INTER[..4], b"wOF2");
        let css = css();
        assert!(css.contains("font-family:Inter") && css.contains("--rx-bg"));
    }
}
