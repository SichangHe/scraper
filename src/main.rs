use anyhow::Result;
use reqwest::Url;
use xxhash_rust::xxh3::xxh3_64;
fn main() -> Result<()> {
    let url = Url::parse("https://google.com")?;
    let hash = xxh3_64(url.as_str().as_bytes());
    let hash_bytes = hash.to_be_bytes();
    let hash_bytes16 = (0..4)
        .map(|i| u16::from_be_bytes([hash_bytes[2 * i], hash_bytes[2 * i + 1]]))
        .collect::<Vec<_>>();
    let hash_str = unsafe { String::from_utf8_unchecked(hash_bytes.to_vec()) };
    let hash_str_utf16 = String::from_utf16(&hash_bytes16)?;
    println!(
        "{url}'s hash is {hash}, correspond to UTF8 string {hash_str},
    and UTF16 string {hash_str_utf16}."
    );
    Ok(())
}
