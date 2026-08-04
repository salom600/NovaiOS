//! User list helpers shared between headless + GUI modes.

pub fn list_human_users() -> Vec<String> {
    let raw = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    raw.lines()
        .filter_map(|l| {
            let mut it = l.split(':');
            let name = it.next()?;
            let uid: u32 = it.nth(1)?.parse().ok()?;
            if uid >= 1000 && uid < 65534 && name != "nobody" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}
