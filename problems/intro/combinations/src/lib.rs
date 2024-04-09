#![forbid(unsafe_code)]

// pub fn combinations(arr: &[i32], k: usize) -> Vec<Vec<i32>> {
//     let mut res: Vec<Vec<i32>> = vec![];
//     let mut acc: Vec<i32> = Vec::with_capacity(k);
//     helper(arr, k, 0, &mut res, &mut acc);
//     res
// }

// fn helper(arr: &[i32], k: usize, idx: usize, res: &mut Vec<Vec<i32>>, acc: &mut Vec<i32>) {
//     if k == arr.len() {
//         res.push(arr.to_vec());
//         return;
//     } else if k == 0 {
//         res.push(acc.clone());
//     } else if idx >= arr.len() || k > arr.len() {
//         return;
//     } else {
//         acc.push(arr[idx]);
//         helper(arr, k - 1, idx + 1, res, acc);
//         acc.pop();
//         helper(arr, k, idx + 1, res, acc);
//     }
// }

pub fn combinations(arr: &[i32], k: usize) -> Vec<Vec<i32>> {
    helper_comb(arr, k)
}
fn helper_comb(arr: &[i32], k: usize) -> Vec<Vec<i32>> {
    if k == 0 {
        return vec![vec![]];
    } else if k == 1 {
        return arr.iter().map(|x| vec![*x]).collect();
    } else if k > arr.len() {
        return vec![];
    }
    let fst_el = arr[0];
    let except_fst = &arr[1..];
    let ret = helper_comb(except_fst, k - 1);
    ret.into_iter()
        .map(|mut v| {
            v.insert(0, fst_el);
            v
        })
        .chain(helper_comb(except_fst, k))
        .collect()
}
