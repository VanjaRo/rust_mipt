#![forbid(unsafe_code)]

pub fn longest_common_prefix(strs: Vec<&str>) -> String {
    if strs.is_empty() {
        return String::from("");
    }
    let mut pref = strs[0];
    // chars_len
    let mut pref_cnt = pref.chars().count();
    for curr_str in strs[1..].iter() {
        if curr_str.is_empty() || pref.is_empty() {
            return String::from("");
        }
        let min_cnt = curr_str.chars().count().min(pref_cnt);
        // due to unicode we should slice to the last byte of unicode symbol
        let mut pref_chrs = pref.char_indices();
        let mut curr_str_chrs = curr_str.chars();
        // we can safely unwrap as we go by the min prefix len
        let mut slice_to = curr_str.len().min(pref.len());
        for i in 0..min_cnt {
            let (offset, char) = pref_chrs.next().unwrap();
            if !char.eq(&(curr_str_chrs.next().unwrap())) {
                slice_to = offset;
                pref_cnt = i;
                break;
            }
        }
        pref = &pref[..slice_to];
    }
    String::from(pref)
}
