
pub type Hash = u32;

#[inline]
pub fn bit_count(n: Hash) -> u32 {
    n.count_ones()
}

#[inline]
pub fn get_bit(n: Hash, mask: Hash) -> bool {
    (n & mask) != 0
}

#[inline]
pub fn set_bit(n: Hash, mask: Hash) -> Hash {
    n | mask
}

#[inline]
pub fn clear_bit(n: Hash, mask: Hash) -> Hash {
    n & !mask
}

fn fast_hash_between(input: &str, begin: usize, end: usize) -> Hash {
    // TODO: replace with another hash function?
    // xxHash is a good candidate:
    //   use fasthash::{xx, XXHasher};
    //   let h = xx::hash64(b"hello world\xff");
    let mut hash: Hash = 5381;
    let mut chars = input.chars().skip(begin).take(end - begin);

    while let Some(c) = chars.next() {
        hash = hash.wrapping_mul(33) ^ (c as Hash);
    }
    hash
}

#[inline]
pub fn fast_hash(input: &str) -> Hash {
    fast_hash_between(&input, 0, input.len())
}

#[inline]
pub fn fast_starts_with(haystack: &str, needle: &str) -> bool {
    haystack.starts_with(needle)
}

#[inline]
pub fn fast_starts_with_from(haystack: &str, needle: &str, start: usize) -> bool {
    haystack[start..].starts_with(needle)
}

#[inline]
fn is_allowed_filter(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '%'
}

#[inline]
fn is_allowed_hostname(ch: char) -> bool {
    is_allowed_filter(ch) || ch == '_' /* '_' */ || ch == '-' /* '-' */
}

pub const TOKENS_BUFFER_SIZE: usize = 200;

fn fast_tokenizer_no_regex(
    pattern: &str,
    is_allowed_code: &Fn(char) -> bool,
    sip_first_token: bool,
    skip_last_token: bool,
) -> Vec<Hash> {
    
    let mut tokens_buffer: [Hash; TOKENS_BUFFER_SIZE] = [0; TOKENS_BUFFER_SIZE];

    let mut tokens_buffer_index = 0;
    let mut inside: bool = false;
    let mut start = 0;
    let mut preceding_ch: Option<char> = None; // Used to check if a '*' is not just before a token
    let mut chars = pattern.char_indices();

    while let Some((i, c)) = chars.next() {
        if tokens_buffer_index >= TOKENS_BUFFER_SIZE {
            break;
        }
        if is_allowed_code(c) {
            if !inside {
                inside = true;
                start = i
            }
        } else if inside {
            inside = false;
            // Should not be followed by '*'
            if (!sip_first_token || start != 0)
                && i - start > 1
                && c != '*'
                && (preceding_ch.is_none() || preceding_ch.unwrap() != '*')
            {
                tokens_buffer[tokens_buffer_index] = fast_hash_between(&pattern, start, i);
                tokens_buffer_index += 1;
            }
            preceding_ch = Some(c)
        } else {
            preceding_ch = Some(c)
        }
        
    }

    if inside
        && pattern.len() - start > 1
        && (preceding_ch.is_none() || preceding_ch.unwrap() != '*')
        && !skip_last_token
    {
        tokens_buffer[tokens_buffer_index] = fast_hash_between(&pattern, start, pattern.len());
        tokens_buffer_index += 1;
    }

    tokens_buffer[0..tokens_buffer_index].to_vec()
}

fn fast_tokenizer(pattern: &str, is_allowed_code: &Fn(char) -> bool) -> Vec<Hash> {
    let mut tokens_buffer: [Hash; TOKENS_BUFFER_SIZE] = [0; TOKENS_BUFFER_SIZE];

    let mut tokens_buffer_index = 0;
    let mut inside: bool = false;
    let mut start = 0;
    let mut chars = pattern.char_indices();

    while let Some((i, c)) = chars.next() {
        if tokens_buffer_index >= TOKENS_BUFFER_SIZE {
            break;
        }
        if is_allowed_code(c) {
            if !inside {
                inside = true;
                start = i;
            }
        } else if inside {
            inside = false;
            tokens_buffer[tokens_buffer_index] = fast_hash_between(&pattern, start, i);
            tokens_buffer_index += 1;
        }
    }

    if inside {
        tokens_buffer[tokens_buffer_index] = fast_hash_between(&pattern, start, pattern.len());
        tokens_buffer_index += 1;
    }

    tokens_buffer[0..tokens_buffer_index].to_vec()
}

#[inline]
pub fn tokenize(pattern: &str) -> Vec<Hash> {
    fast_tokenizer_no_regex(pattern, &is_allowed_filter, false, false)
}

#[inline]
pub fn tokenize_filter(pattern: &str, sip_first_token: bool, skip_last_token: bool) -> Vec<Hash> {
    fast_tokenizer_no_regex(pattern, &is_allowed_filter, sip_first_token, skip_last_token)
}

#[inline]
pub fn tokenize_hostnames(pattern: &str) -> Vec<Hash> {
    fast_tokenizer(&pattern, &is_allowed_hostname)
}

fn compact_tokens<T: std::cmp::Ord>(tokens: &mut Vec<T>) {
    tokens.sort_unstable();
    tokens.dedup();
}

#[inline]
pub fn create_fuzzy_signature(pattern: &str) -> Vec<Hash> {
    let mut tokens = fast_tokenizer(pattern, &is_allowed_filter);
    compact_tokens(&mut tokens);
    tokens
}

pub fn bin_search<T: Ord>(arr: &[T], elt: T) -> Option<usize> {
    arr.binary_search(&elt).ok()
}

pub fn bin_lookup<T: Ord>(arr: &[T], elt: T) -> bool {
    arr.binary_search(&elt).is_ok()
}

pub fn has_unicode(pattern: &str) -> bool {
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if !c.is_ascii() {
            return true
        }
    }
    return false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_count_works() {
        assert_eq!(bit_count(0b0011u32), 2);
        assert_eq!(bit_count(0b10110011u32), 5);
        assert_eq!(bit_count(std::u32::MAX), 32);
        assert_eq!(bit_count(0), 0);
    }

    #[test]
    fn get_bit_works() {
        assert_eq!(get_bit(0b0011u32, 0b0011u32), true);
        assert_eq!(get_bit(0b10110011u32, 0b10110011u32), true);
        assert_eq!(get_bit(0b10110011u32, 0b00100000u32), true);
        assert_eq!(get_bit(0b10110011u32, 0b01001100u32), false);
        assert_eq!(get_bit(0, std::u32::MAX), false);
    }

    #[test]
    fn set_bit_works() {
        assert_eq!(set_bit(0b0011u32, 0b0011u32), 0b0011u32);
        assert_eq!(set_bit(0b10110011u32, 0b100u32), 0b10110111u32);
        assert_eq!(set_bit(0b10110011u32, 0), 0b10110011u32);
        assert_eq!(set_bit(std::u32::MAX, 0), std::u32::MAX);
        assert_eq!(set_bit(0, std::u32::MAX), std::u32::MAX);
    }

    #[test]
    fn clear_bit_works() {
        assert_eq!(clear_bit(0b10110011u32, 0b1), 0b10110010u32);
        assert_eq!(clear_bit(0b10110011u32, 0b100u32), 0b10110011u32);
        assert_eq!(clear_bit(0b10110011u32, 0b10110011u32), 0);
        assert_eq!(clear_bit(std::u32::MAX, std::u32::MAX), 0);
        assert_eq!(clear_bit(std::u32::MAX, 0), std::u32::MAX);
        assert_eq!(clear_bit(0, std::u32::MAX), 0);
    }

    #[test]
    fn fast_hash_works() {
        assert_eq!(fast_hash("hello world"), 4173747013); // cross-checked with the TS implementation
        assert_eq!(fast_hash("ello worl"), 2759317833); // cross-checked with the TS implementation
        assert_eq!(
            fast_hash_between("hello world", 1, 10),
            fast_hash("ello worl")
        );
        assert_eq!(fast_hash_between("hello world", 1, 5), fast_hash("ello"));
    }

    #[test]
    fn fast_starts_with_from_works() {
        assert_eq!(fast_starts_with_from("hello world", "hello", 0), true);
        assert_eq!(fast_starts_with_from("hello world", "hello", 1), false);
        assert_eq!(fast_starts_with_from("hello", "hello world", 1), false);
        assert_eq!(fast_starts_with_from("hello world", " world", 5), true);
    }

    fn t(tokens: &[&str]) -> Vec<Hash> {
        tokens.into_iter().map(|t| fast_hash(&t)).collect()
    }

    #[test]
    fn tokenize_filter_works() {
        assert_eq!(
            tokenize_filter("", false, false).as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", true, false).as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", false, true).as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", true, true).as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", false, false).as_slice(),
            t(&vec![]).as_slice()
        );

        assert_eq!(
            tokenize_filter("foo/bar baz", false, false).as_slice(),
            t(&vec!["foo", "bar", "baz"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo/bar baz", true, false).as_slice(),
            t(&vec!["bar", "baz"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo/bar baz", true, true).as_slice(),
            t(&vec!["bar"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo/bar baz", false, true).as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo////bar baz", false, true).as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
    }

    #[test]
    fn tokenize_host_works() {
        assert_eq!(
            tokenize_hostnames("").as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize_hostnames("foo").as_slice(),
            t(&vec!["foo"]).as_slice()
        );
        assert_eq!(
            tokenize_hostnames("foo/bar").as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize_hostnames("foo-barbaz/bar").as_slice(),
            t(&vec!["foo-barbaz", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize_hostnames("foo_barbaz/ba%r").as_slice(),
            t(&vec!["foo_barbaz", "ba%r"]).as_slice()
        );

        assert_eq!(
            tokenize_hostnames("foo_barbaz/ba%r*").as_slice(),
            t(&vec!["foo_barbaz", "ba%r"]).as_slice()
        );
        assert_eq!(
            tokenize_hostnames("foo_barbaz///ba%r*").as_slice(),
            t(&vec!["foo_barbaz", "ba%r"]).as_slice()
        );
    }

    #[test]
    fn tokenize_works() {
        assert_eq!(
            tokenize("").as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize("foo").as_slice(),
            t(&vec!["foo"]).as_slice()
        );
        assert_eq!(
            tokenize("foo/bar").as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize("foo-bar").as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize("foo.bar").as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize("foo.barƬ").as_slice(),
            t(&vec!["foo", "barƬ"]).as_slice()
        );

        // Tokens cannot be surrounded by *
        assert_eq!(
            tokenize("foo.barƬ*").as_slice(),
            t(&vec!["foo"]).as_slice()
        );
        assert_eq!(
            tokenize("*foo.barƬ").as_slice(),
            t(&vec!["barƬ"]).as_slice()
        );
        assert_eq!(
            tokenize("*foo.barƬ*").as_slice(),
            t(&vec![]).as_slice()
        );
    }

    #[test]
    fn create_fuzzy_signature_works() {
        assert_eq!(create_fuzzy_signature("").as_slice(), t(&vec![]).as_slice());
        assert_eq!(create_fuzzy_signature("foo bar").as_slice(), t(&vec!["foo", "bar"]).as_slice());
        assert_eq!(create_fuzzy_signature("bar foo").as_slice(), t(&vec!["foo", "bar"]).as_slice());
        assert_eq!(create_fuzzy_signature("foo bar foo foo").as_slice(), t(&vec!["foo", "bar"]).as_slice());
    }

    #[test]
    fn bin_lookup_works() {
        assert_eq!(bin_lookup(&vec![], 42), false);
        assert_eq!(bin_lookup(&vec![42], 42), true);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 42), true);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 1), true);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 3), true);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 43), false);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 0), false);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 5), false);
    }

    #[test]
    fn bin_search_works() {
        // empty array
        assert_eq!(bin_search(&Vec::new(), 42), None);
        // array of length 1
        assert_eq!(bin_search(&vec![1], 42), None);
        assert_eq!(bin_search(&vec![42], 42), Some(0));
        // array of length 2
        assert_eq!(bin_search(&vec![0, 1], 42), None);
        assert_eq!(bin_search(&vec![1, 42], 42), Some(1));
        assert_eq!(bin_search(&vec![42, 45], 42), Some(0));
        assert_ne!(bin_search(&vec![42, 42], 42), None);

        // bigger arrays
        let data : Vec<Hash> = (1u32..=1000).map(|x| x*x).collect();
        assert_eq!(bin_search(&data, 42), None);
        assert_eq!(bin_search(&data, 1), Some(0));
        assert_eq!(bin_search(&data, 4), Some(1));
        assert_eq!(bin_search(&data, 1000*1000), Some(1000-1));
    }

    #[test]
    fn has_unicode_works() {
        let ascii: String = (b'!'..=b'~') // Start as u8
        .map(|c| c as char)
        .collect();

        assert_eq!(has_unicode(&ascii), false);
        assert_eq!(has_unicode("｡◕ ∀ ◕｡)"), true);
        assert_eq!(has_unicode("｀ｨ(´∀｀∩"), true);
        assert_eq!(has_unicode("__ﾛ(,_,*)"), true);
        assert_eq!(has_unicode("・(￣∀￣)・:*:"), true);
        assert_eq!(has_unicode("ﾟ･✿ヾ╲(｡◕‿◕｡)╱✿･ﾟ"), true);
        assert_eq!(has_unicode(",。・:*:・゜’( ☻ ω ☻ )。・:*:・゜’"), true);
        assert_eq!(has_unicode("(╯°□°）╯︵ ┻━┻)"), true);
        assert_eq!(has_unicode("(ﾉಥ益ಥ）ﾉ ┻━┻"), true);
        assert_eq!(has_unicode("┬─┬ノ( º _ ºノ)"), true);
        assert_eq!(has_unicode("( ͡° ͜ʖ ͡°)"), true);
        assert_eq!(has_unicode("¯_(ツ)_/¯"), true);
    }

    // TODO: check for collisions in filters/urls
}
