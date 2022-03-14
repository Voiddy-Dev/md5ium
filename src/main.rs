use rand::Rng;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

const IV: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
const RELATIVE_INDEX: usize = 4;

// Step-dependent constant values
const SMAP: [i32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];
const MMAP: [u8; 64] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 6, 11, 0, 5, 10, 15, 4, 9, 14, 3, 8,
    13, 2, 7, 12, 5, 8, 11, 14, 1, 4, 7, 10, 13, 0, 3, 6, 9, 12, 15, 2, 0, 7, 14, 5, 12, 3, 10, 1,
    8, 15, 6, 13, 4, 11, 2, 9,
];
const TMAP: [u32; 64] = [
    /* Round 1 */
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    /* Round 2 */
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x2441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    /* Round 3 */
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x4881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    /* Round 4 */
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

const DIFFERENCES: [u32; 64] = [
    0x82000000, 0x82000020, 0xfe3f18e0, 0x8600003e, 0x80001fc1, 0x80330000, 0x980003c0, 0x87838000,
    0x800003c3, 0x80001000, 0x80000000, 0x800fe080, 0xff000000, 0x80000000, 0x80008008, 0xa0000000,
    0x80000000, 0x80000000, 0x80020000, 0x80000000, 0x80000000, 0x80000000, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000,
    0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000,
    0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000,
    0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x82000000, 0x82000000, 0x86000000,
];

#[inline]
fn addsub_bit(x: u32, i: i32, b: i32) -> u32 {
    if b != 1 && b != -1 {
        panic!("b is not 1 or -1");
    }
    if i < 0 {
        panic!("Negative i");
    }
    let (res, overflow) = (1i32).overflowing_shl(i as u32);
    let t;
    if overflow {
        t = 0;
    } else {
        (t, _) = b.overflowing_mul(res);
    }
    // x + t
    let (return_val, _) = x.overflowing_add(t as u32);
    return_val
}

#[inline]
fn fix_n19(g_n19: &mut u32) {
    if get_bit(*g_n19, 12) == 1 {
        *g_n19 = addsub_bit(*g_n19, 12, 1);
    }
    if get_bit(*g_n19, 26) == 1 {
        *g_n19 = addsub_bit(*g_n19, 26, 1);
    }
}

#[inline]
fn cls(x: u32, s: i32) -> u32 {
    (x << s) ^ (x >> (32 - s))
}
#[inline]
fn crs(x: u32, s: i32) -> u32 {
    (x >> s) ^ (x << (32 - s))
}

#[inline]
fn get_bit(x: u32, i: i32) -> u32 {
    return (x >> i) & 1;
}
#[inline]
fn set_bit(x: u32, i: i32, b: i32) -> u32 {
    if b == 1 {
        return x | (1 << i);
    } else {
        if get_bit(x, i) == 0 {
            return x;
        } else {
            return x - (1 << i);
        }
    }
}

// Round functions for MD5
#[inline]
fn md5_f(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (!x & z)
}
#[inline]
fn md5_g(x: u32, y: u32, z: u32) -> u32 {
    return (x & z) | (y & (!z));
}
#[inline]
fn md5_h(x: u32, y: u32, z: u32) -> u32 {
    return x ^ y ^ z;
}
#[inline]
fn md5_i(x: u32, y: u32, z: u32) -> u32 {
    return y ^ (x | (!z));
}

#[inline]
fn cover_func(b: u32, c: u32, d: u32, i: usize) -> u32 {
    if i < 16 {
        return md5_f(b, c, d);
    }
    if i < 32 {
        return md5_g(b, c, d);
    }
    if i < 48 {
        return md5_h(b, c, d);
    }
    return md5_i(b, c, d);
}

#[inline]
fn phi(q_cond_nodes: &mut [u32; 68], i: usize) -> u32 {
    return cover_func(
        q_cond_nodes[i - 1],
        q_cond_nodes[i - 2],
        q_cond_nodes[i - 3],
        i - 4,
    );
}

#[derive(Debug)]
struct Condition {
    ind: i32,
    cref: i32,
    crind: i32,
    add_const: i32,
}

#[derive(Debug)]
struct Node {
    val: u32,
    tval: u32,
    bf: [u32; 4],
    list: Vec<Condition>,
}

impl Default for Node {
    fn default() -> Node {
        Node {
            val: 0,
            tval: 0,
            bf: [0, 0, 0, 0],
            list: Vec::new(),
        }
    }
}

fn smm5(index: i32, n_cond_nodes: &mut Vec<Node>) -> u32 {
    let mut y: u32;
    let mut b2: i32;
    let mut i1: i32;
    let mut i2: i32;
    let mut i3: i32;
    let mut i4: i32;

    let mut x = n_cond_nodes[RELATIVE_INDEX + index as usize].val;
    for el in &n_cond_nodes[RELATIVE_INDEX + index as usize].list {
        if el.cref < 0 {
            x = set_bit(x, el.ind, el.cref + 2);
        } else {
            y = n_cond_nodes[RELATIVE_INDEX + el.cref as usize].val;
            b2 = get_bit(y, el.crind) as i32;
            x = set_bit(x, el.ind, b2);
        }
    }
    n_cond_nodes[RELATIVE_INDEX + index as usize].val = x;
    i1 = index - 1;
    i2 = index - 2;
    i3 = index - 3;
    i4 = index - 4;
    if i1 < 0 {
        i1 += 68;
    }
    if i2 < 0 {
        i2 += 68;
    }
    if i3 < 0 {
        i3 += 68;
    }
    if i4 < 0 {
        i4 += 68;
    }
    // recompute correct message value for updated value of x
    return crs(
        x.overflowing_sub(n_cond_nodes[RELATIVE_INDEX + i1 as usize].val)
            .0,
        SMAP[index as usize],
    )
    .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + i4 as usize].val)
    .0
    .overflowing_sub(md5_f(
        n_cond_nodes[RELATIVE_INDEX + i1 as usize].val,
        n_cond_nodes[RELATIVE_INDEX + i2 as usize].val,
        n_cond_nodes[RELATIVE_INDEX + i3 as usize].val,
    ))
    .0
    .overflowing_sub(TMAP[index as usize])
    .0;
}

fn build_bitfield(n_cond_nodes: &mut Vec<Node>) {
    let mut count = 0;
    for el in n_cond_nodes {
        if count >= RELATIVE_INDEX {
            let mut list_iter = el.list.iter();
            while let Some(li) = list_iter.next() {
                if li.cref == -1 {
                    el.bf[0] = addsub_bit(el.bf[0], li.ind, 1);
                }
                if li.cref == -2 {
                    el.bf[1] = addsub_bit(el.bf[1], li.ind, 1);
                }
                if (li.cref > -1) && (li.add_const == 0) {
                    el.bf[2] = addsub_bit(el.bf[2], li.crind, 1)
                }
                if (li.cref > -1) && (li.add_const != 0) {
                    el.bf[3] = addsub_bit(el.bf[3], li.crind, 1);
                }
            }
        }
        count += 1;
    }
}

fn build_condition_list_block_2(filename: String) -> [[u32; 3]; 309] {
    let f = File::open(filename).expect("Errors reading cond file");
    let reader = BufReader::new(f);

    let mut res: [[u32; 3]; 309] = [[0; 3]; 309];

    for (i, line) in reader.lines().enumerate() {
        match line {
            Ok(l) => {
                let mut split = l.split(" ");
                res[i][0] = split.next().unwrap().parse::<u32>().unwrap();
                res[i][1] = split.next().unwrap().parse::<u32>().unwrap();
                res[i][2] = split.next().unwrap().parse::<u32>().unwrap();
            }
            _ => print!("Error in line."),
        }
    }
    res
}

fn build_condition_list(filename: String) -> Vec<Node> {
    let f = File::open(filename).expect("Errors reading cond file");
    let reader = BufReader::new(f);

    let mut res: Vec<Node> = Vec::new();
    for _ in 0..76 {
        res.push(Node::default())
    }

    for line in reader.lines() {
        match line {
            Ok(l) => {
                let mut split = l.split(" ");
                // Getting index
                let mut q_index: usize = split.next().unwrap().parse().unwrap();
                q_index += RELATIVE_INDEX;
                assert!(q_index < 76);

                // init condition
                let cond = Condition {
                    ind: split.next().unwrap().parse().unwrap(),
                    cref: split.next().unwrap().parse().unwrap(),
                    crind: split.next().unwrap().parse().unwrap(),
                    add_const: split.next().unwrap().parse().unwrap(),
                };
                res[q_index].list.push(cond);
                // Sorting by index
                res[q_index].list.sort_by(|b, a| b.ind.cmp(&a.ind));
            }
            _ => print!("Error in line."),
        }
    }
    // returning
    res
}

fn construct_diff_table() -> [u32; 68] {
    let mut diff_table: [u32; 68] = [0; 68];
    diff_table[0] = 0;
    diff_table[1] = 0;
    diff_table[2] = 0;
    diff_table[3] = 0;
    diff_table[4] = addsub_bit(0, 6, -1);
    diff_table[5] = addsub_bit(0, 6, -1);
    diff_table[5] = addsub_bit(diff_table[5], 23, 1);
    diff_table[5] = addsub_bit(diff_table[5], 31, 1);
    diff_table[6] = addsub_bit(0, 6, -1);
    diff_table[6] = addsub_bit(diff_table[6], 23, 1);
    diff_table[6] -= 1;
    diff_table[6] = diff_table[6].wrapping_sub(addsub_bit(0, 27, 1));
    diff_table[7] += 1;
    diff_table[7] = diff_table[7].wrapping_sub(addsub_bit(0, 15, 1));
    diff_table[7] = diff_table[7].wrapping_sub(addsub_bit(0, 17, 1));
    diff_table[7] = diff_table[7].wrapping_sub(addsub_bit(0, 23, 1));
    diff_table[8] += 1;
    diff_table[8] = diff_table[8].wrapping_sub(addsub_bit(0, 6, 1));
    diff_table[8] = diff_table[8].wrapping_add(addsub_bit(0, 31, 1));
    diff_table[9] += addsub_bit(0, 12, 1);
    diff_table[9] += addsub_bit(0, 31, 1);
    diff_table[10] += addsub_bit(0, 31, 1);
    diff_table[10] += addsub_bit(0, 30, 1);
    diff_table[11] += addsub_bit(0, 31, 1);
    diff_table[11] -= addsub_bit(0, 7, 1);
    diff_table[11] -= addsub_bit(0, 13, 1);
    diff_table[12] += addsub_bit(0, 24, 1);
    diff_table[12] += addsub_bit(0, 31, 1);
    diff_table[13] += addsub_bit(0, 31, 1);
    diff_table[14] += addsub_bit(0, 31, 1);
    diff_table[14] += addsub_bit(0, 3, 1);
    diff_table[14] -= addsub_bit(0, 15, 1);
    diff_table[15] += addsub_bit(0, 31, 1);
    diff_table[15] -= addsub_bit(0, 29, 1);
    diff_table[16] += addsub_bit(0, 31, 1);
    diff_table[17] += addsub_bit(0, 31, 1);
    diff_table[18] += addsub_bit(0, 31, 1);
    diff_table[18] += addsub_bit(0, 17, 1);
    diff_table[19] += addsub_bit(0, 31, 1);
    diff_table[20] += addsub_bit(0, 31, 1);
    diff_table[21] += addsub_bit(0, 31, 1);
    diff_table[48] += addsub_bit(0, 31, 1);
    diff_table[49] += addsub_bit(0, 31, -1);
    diff_table[50] += addsub_bit(0, 31, 1);
    diff_table[51] += addsub_bit(0, 31, -1);
    diff_table[52] += addsub_bit(0, 31, -1);
    diff_table[53] += addsub_bit(0, 31, -1);
    diff_table[54] += addsub_bit(0, 31, -1);
    diff_table[55] += addsub_bit(0, 31, -1);
    diff_table[56] += addsub_bit(0, 31, -1);
    diff_table[57] += addsub_bit(0, 31, -1);
    diff_table[58] += addsub_bit(0, 31, 1);
    diff_table[59] += addsub_bit(0, 31, 1);
    diff_table[60] += addsub_bit(0, 31, 1);
    diff_table[61] += addsub_bit(0, 32, 1);
    diff_table[61] += addsub_bit(0, 25, 1);
    diff_table[62] += addsub_bit(0, 31, 1);
    diff_table[62] += addsub_bit(0, 25, 1);
    diff_table[63] += addsub_bit(0, 31, -1);
    diff_table[63] += addsub_bit(0, 25, 1);
    diff_table[64] += addsub_bit(0, 31, 1); // these last four values in diff_table
    diff_table[65] += addsub_bit(0, 31, 1); // contain the needed differentials for the
    diff_table[65] += addsub_bit(0, 25, 1); // chaining variables
    diff_table[66] += addsub_bit(0, 31, 1);
    diff_table[66] += addsub_bit(0, 25, 1);
    diff_table[67] += addsub_bit(0, 31, -1);
    diff_table[67] += addsub_bit(0, 25, 1);

    diff_table
}

fn first_round(m_block: &mut [u32; 32], n_cond_nodes: &mut Vec<Node>, diff_table: [u32; 68]) {
    let mut flag: i32 = 0;

    while flag == 0 {
        flag = 1;

        for i in 0..16 {
            // Do initial computation
            n_cond_nodes[RELATIVE_INDEX + i].val = n_cond_nodes[RELATIVE_INDEX + i - 1]
                .val
                .overflowing_add(cls(
                    md5_f(
                        n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                    )
                    .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                    .0
                    .overflowing_add(m_block[i])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;
            m_block[i] = smm5(i as i32, n_cond_nodes);
            n_cond_nodes[RELATIVE_INDEX + i].val = n_cond_nodes[RELATIVE_INDEX + i - 1]
                .val
                .overflowing_add(cls(
                    md5_f(
                        n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                    )
                    .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                    .0
                    .overflowing_add(m_block[i])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;
        }
        m_block[4] = addsub_bit(m_block[4], 31, 1);
        m_block[11] = addsub_bit(m_block[11], 15, 1);
        m_block[14] = addsub_bit(m_block[14], 31, 1);

        for i in 0..16 {
            n_cond_nodes[RELATIVE_INDEX + i].tval = n_cond_nodes[RELATIVE_INDEX + i - 1]
                .tval
                .overflowing_add(cls(
                    md5_f(
                        n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                        n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                        n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                    )
                    .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                    .0
                    .overflowing_add(m_block[i])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;

            if n_cond_nodes[RELATIVE_INDEX + i]
                .tval
                .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + i].val)
                .0
                != diff_table[i]
            {
                flag = 0;
                new_rand_mblock(m_block);
            }
        }
        m_block[4] = addsub_bit(m_block[4], 31, -1);
        m_block[11] = addsub_bit(m_block[11], 15, -1);
        m_block[14] = addsub_bit(m_block[14], 31, -1);
    }
}

fn new_rand_mblock(m_block: &mut [u32; 32]) {
    let mut temp: [u32; 32] = [0; 32];
    temp.copy_from_slice(m_block);
    let mut rng = rand::thread_rng();
    for i in 0..16 {
        m_block[i] = rng.gen();
    }
    assert_ne!(&mut temp, m_block);
}

fn fcheck_cond(ind: i32, n_cond_nodes: &mut Vec<Node>) -> u32 {
    let mut x: u32 = 0;
    x |= (!n_cond_nodes[RELATIVE_INDEX + ind as usize].val)
        & n_cond_nodes[RELATIVE_INDEX + ind as usize].bf[0];
    x |= n_cond_nodes[RELATIVE_INDEX + ind as usize].val
        & n_cond_nodes[RELATIVE_INDEX + ind as usize].bf[1];
    x |= (n_cond_nodes[RELATIVE_INDEX + ind as usize - 1].val
        & n_cond_nodes[RELATIVE_INDEX + ind as usize].bf[2])
        ^ (n_cond_nodes[RELATIVE_INDEX + ind as usize].val
            & n_cond_nodes[RELATIVE_INDEX + ind as usize].bf[2]);

    if n_cond_nodes[RELATIVE_INDEX + ind as usize].bf[3] != 0 {
        let list_iter = n_cond_nodes[RELATIVE_INDEX + ind as usize].list.iter();
        let li = list_iter.last();
        match li {
            Some(list) => {
                x |= (!(n_cond_nodes[list.crind as usize].val)
                    & n_cond_nodes[RELATIVE_INDEX + ind as usize].bf[3])
                    ^ (n_cond_nodes[RELATIVE_INDEX + ind as usize].val
                        & n_cond_nodes[RELATIVE_INDEX + ind as usize].bf[2]);
            }
            _ => {
                panic!("BRUV LI MUST BE SOME");
            }
        }
    }
    x
}

fn klima1_3(m_block: &mut [u32; 32], n_cond_nodes: &mut Vec<Node>) -> bool {
    let mut rng = rand::thread_rng();
    let mut x: u32;

    n_cond_nodes[RELATIVE_INDEX + 17].val = 0;
    let mut count = 0;
    while (fcheck_cond(17, n_cond_nodes) != 0) || (fcheck_cond(18, n_cond_nodes) != 0) {
        count += 1;
        if count > 4096 {
            return true;
        }

        n_cond_nodes[RELATIVE_INDEX + 16].val = rng.gen();
        x = n_cond_nodes[RELATIVE_INDEX + 16].val;
        for (_, list) in n_cond_nodes[RELATIVE_INDEX + 16 as usize]
            .list
            .iter()
            .enumerate()
        {
            if list.cref < 0 {
                x = set_bit(x, list.ind, list.cref + 2);
            } else {
                x = set_bit(
                    x,
                    list.ind,
                    get_bit(
                        n_cond_nodes[RELATIVE_INDEX + list.cref as usize].val,
                        list.crind,
                    ) as i32,
                );
            }
        }

        n_cond_nodes[RELATIVE_INDEX + 16].val = x;
        n_cond_nodes[RELATIVE_INDEX + 17].val = n_cond_nodes[RELATIVE_INDEX + 16]
            .val
            .overflowing_add(cls(
                md5_g(
                    n_cond_nodes[RELATIVE_INDEX + 16].val,
                    n_cond_nodes[RELATIVE_INDEX + 15].val,
                    n_cond_nodes[RELATIVE_INDEX + 14].val,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + 13].val)
                .0
                .overflowing_add(m_block[6])
                .0
                .overflowing_add(TMAP[17])
                .0,
                SMAP[17],
            ))
            .0;
        n_cond_nodes[RELATIVE_INDEX + 18].val = n_cond_nodes[RELATIVE_INDEX + 17]
            .val
            .overflowing_add(cls(
                md5_g(
                    n_cond_nodes[RELATIVE_INDEX + 17].val,
                    n_cond_nodes[RELATIVE_INDEX + 16].val,
                    n_cond_nodes[RELATIVE_INDEX + 15].val,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + 14].val)
                .0
                .overflowing_add(m_block[11])
                .0
                .overflowing_add(TMAP[18])
                .0,
                SMAP[18],
            ))
            .0;
    }
    false
}

fn klima4_9(m_block: &mut [u32; 32], n_cond_nodes: &mut Vec<Node>, g_n19: &mut u32) {
    n_cond_nodes[RELATIVE_INDEX + 19].val = *g_n19;
    *g_n19 += 1;
    fix_n19(g_n19);

    m_block[0] = crs(
        n_cond_nodes[RELATIVE_INDEX + 19]
            .val
            .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 18].val)
            .0,
        20,
    )
    .overflowing_sub(md5_g(
        n_cond_nodes[RELATIVE_INDEX + 18].val,
        n_cond_nodes[RELATIVE_INDEX + 17].val,
        n_cond_nodes[RELATIVE_INDEX + 16].val,
    ))
    .0
    .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 15].val)
    .0
    .overflowing_sub(0xe9b6c7aa)
    .0;
    n_cond_nodes[RELATIVE_INDEX + 0].val = n_cond_nodes[RELATIVE_INDEX + 67]
        .val
        .overflowing_add(cls(
            m_block[0]
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + 64].val)
                .0
                .overflowing_add(md5_f(
                    n_cond_nodes[RELATIVE_INDEX + 67].val,
                    n_cond_nodes[RELATIVE_INDEX + 66].val,
                    n_cond_nodes[RELATIVE_INDEX + 65].val,
                ))
                .0
                .overflowing_add(TMAP[0])
                .0,
            SMAP[0],
        ))
        .0;
    m_block[1] = crs(
        n_cond_nodes[RELATIVE_INDEX + 16]
            .val
            .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 15].val)
            .0,
        5,
    )
    .overflowing_sub(md5_g(
        n_cond_nodes[RELATIVE_INDEX + 15].val,
        n_cond_nodes[RELATIVE_INDEX + 14].val,
        n_cond_nodes[RELATIVE_INDEX + 13].val,
    ))
    .0
    .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 12].val)
    .0
    .overflowing_sub(0xf61e2562)
    .0;
    n_cond_nodes[RELATIVE_INDEX + 1].val = n_cond_nodes[RELATIVE_INDEX + 0]
        .val
        .overflowing_add(cls(
            m_block[1]
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + 65].val)
                .0
                .overflowing_add(md5_f(
                    n_cond_nodes[RELATIVE_INDEX + 0].val,
                    n_cond_nodes[RELATIVE_INDEX + 67].val,
                    n_cond_nodes[RELATIVE_INDEX + 66].val,
                ))
                .0
                .overflowing_add(TMAP[1])
                .0,
            SMAP[1],
        ))
        .0;
    m_block[2] = crs(
        n_cond_nodes[RELATIVE_INDEX + 2]
            .val
            .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 1].val)
            .0,
        17,
    )
    .overflowing_sub(md5_f(
        n_cond_nodes[RELATIVE_INDEX + 1].val,
        n_cond_nodes[RELATIVE_INDEX + 0].val,
        n_cond_nodes[RELATIVE_INDEX + 67].val,
    ))
    .0
    .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 66].val)
    .0
    .overflowing_sub(TMAP[2])
    .0;
    m_block[3] = crs(
        n_cond_nodes[RELATIVE_INDEX + 3]
            .val
            .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 2].val)
            .0,
        22,
    )
    .overflowing_sub(md5_f(
        n_cond_nodes[RELATIVE_INDEX + 2].val,
        n_cond_nodes[RELATIVE_INDEX + 1].val,
        n_cond_nodes[RELATIVE_INDEX + 0].val,
    ))
    .0
    .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 67].val)
    .0
    .overflowing_sub(TMAP[3])
    .0;
    m_block[4] = crs(
        n_cond_nodes[RELATIVE_INDEX + 4]
            .val
            .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 3].val)
            .0,
        7,
    )
    .overflowing_sub(md5_f(
        n_cond_nodes[RELATIVE_INDEX + 3].val,
        n_cond_nodes[RELATIVE_INDEX + 2].val,
        n_cond_nodes[RELATIVE_INDEX + 1].val,
    ))
    .0
    .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 0].val)
    .0
    .overflowing_sub(TMAP[4])
    .0;
    m_block[5] = crs(
        n_cond_nodes[RELATIVE_INDEX + 5]
            .val
            .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 4].val)
            .0,
        12,
    )
    .overflowing_sub(md5_f(
        n_cond_nodes[RELATIVE_INDEX + 4].val,
        n_cond_nodes[RELATIVE_INDEX + 3].val,
        n_cond_nodes[RELATIVE_INDEX + 2].val,
    ))
    .0
    .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + 1].val)
    .0
    .overflowing_sub(TMAP[5])
    .0;
    n_cond_nodes[RELATIVE_INDEX + 20].val = n_cond_nodes[RELATIVE_INDEX + 19]
        .val
        .overflowing_add(cls(
            m_block[5]
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + 16].val)
                .0
                .overflowing_add(md5_g(
                    n_cond_nodes[RELATIVE_INDEX + 19].val,
                    n_cond_nodes[RELATIVE_INDEX + 18].val,
                    n_cond_nodes[RELATIVE_INDEX + 17].val,
                ))
                .0
                .overflowing_add(0xd62f105d)
                .0,
            5,
        ))
        .0;
    if fcheck_cond(20, n_cond_nodes) != 0 {
        *g_n19 += 0x7f;
        fix_n19(g_n19);
        n_cond_nodes[RELATIVE_INDEX + 19].val = *g_n19;
    }
}

fn first_block(
    m_block: &mut [u32; 32],
    n_cond_nodes: &mut Vec<Node>,
    dt: [u32; 68],
    g_n19: &mut u32,
) {
    n_cond_nodes[RELATIVE_INDEX + 64].val = IV[0];
    n_cond_nodes[RELATIVE_INDEX - 4].val = IV[0];
    n_cond_nodes[RELATIVE_INDEX - 4].tval = IV[0];

    n_cond_nodes[RELATIVE_INDEX + 65].val = IV[3];
    n_cond_nodes[RELATIVE_INDEX - 3].val = IV[3];
    n_cond_nodes[RELATIVE_INDEX - 3].tval = IV[3];

    n_cond_nodes[RELATIVE_INDEX + 66].val = IV[2];
    n_cond_nodes[RELATIVE_INDEX - 2].val = IV[2];
    n_cond_nodes[RELATIVE_INDEX - 2].tval = IV[2];

    n_cond_nodes[RELATIVE_INDEX + 67].val = IV[1];
    n_cond_nodes[RELATIVE_INDEX - 1].val = IV[1];
    n_cond_nodes[RELATIVE_INDEX - 1].tval = IV[1];

    first_round(m_block, n_cond_nodes, dt);

    while klima1_3(m_block, n_cond_nodes) {
        new_rand_mblock(m_block);
        first_round(m_block, n_cond_nodes, dt);
    }
    klima4_9(m_block, n_cond_nodes, g_n19);
    let mut stepno = check_diffs(m_block, n_cond_nodes, 20, dt);

    while stepno >= 0 {
        if *g_n19 >= 0x80000000 {
            *g_n19 = 0;
            while klima1_3(m_block, n_cond_nodes) {
                new_rand_mblock(m_block);
                first_round(m_block, n_cond_nodes, dt);
            }
        }
        klima4_9(m_block, n_cond_nodes, g_n19);
        stepno = check_diffs(m_block, n_cond_nodes, 20, dt);
    }
}

fn check_diffs(
    m_block: &mut [u32; 32],
    n_cond_nodes: &mut Vec<Node>,
    index: i32,
    dt: [u32; 68],
) -> i32 {
    let mut m_prime_block: [u32; 16] = [0; 16];
    m_prime_block.copy_from_slice(&m_block[..16]);

    for i in 0..16 {
        assert_eq!(m_block[i], m_prime_block[i]);
    }

    m_prime_block[4] = addsub_bit(m_prime_block[4], 31, 1);
    m_prime_block[11] = addsub_bit(m_prime_block[11], 15, 1);
    m_prime_block[14] = addsub_bit(m_prime_block[14], 31, 1);
    let mut local_index: usize = index as usize;
    if local_index == 20 {
        for i in 15..20 {
            n_cond_nodes[RELATIVE_INDEX + i].tval = n_cond_nodes[RELATIVE_INDEX + i]
                .val
                .overflowing_add(dt[i])
                .0;
        }
    }

    if local_index != 20 {
        for i in 0..16 {
            n_cond_nodes[RELATIVE_INDEX + i].val = n_cond_nodes[RELATIVE_INDEX + i - 1]
                .val
                .overflowing_add(cls(
                    md5_f(
                        n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                    )
                    .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                    .0
                    .overflowing_add(m_block[MMAP[i] as usize])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;

            n_cond_nodes[RELATIVE_INDEX + i].tval = n_cond_nodes[RELATIVE_INDEX + i - 1]
                .tval
                .overflowing_add(cls(
                    md5_f(
                        n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                        n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                        n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                    )
                    .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                    .0
                    .overflowing_add(m_prime_block[MMAP[i] as usize])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;

            if n_cond_nodes[RELATIVE_INDEX + i]
                .tval
                .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + i].val)
                .0
                != dt[i]
            {
                return i as i32;
            }
        }
        local_index = 16;
    }

    for i in local_index..32 {
        n_cond_nodes[RELATIVE_INDEX + i].val = n_cond_nodes[RELATIVE_INDEX + i - 1]
            .val
            .overflowing_add(cls(
                md5_g(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                .0
                .overflowing_add(m_block[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        n_cond_nodes[RELATIVE_INDEX + i].tval = n_cond_nodes[RELATIVE_INDEX + i - 1]
            .tval
            .overflowing_add(cls(
                md5_g(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                .0
                .overflowing_add(m_prime_block[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;
        if n_cond_nodes[RELATIVE_INDEX + i]
            .tval
            .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + i].val)
            .0
            != dt[i]
        {
            return i as i32;
        }
    }

    for i in 32..48 {
        n_cond_nodes[RELATIVE_INDEX + i].val = n_cond_nodes[RELATIVE_INDEX + i - 1]
            .val
            .overflowing_add(cls(
                md5_h(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                .0
                .overflowing_add(m_block[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        n_cond_nodes[RELATIVE_INDEX + i].tval = n_cond_nodes[RELATIVE_INDEX + i - 1]
            .tval
            .overflowing_add(cls(
                md5_h(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                .0
                .overflowing_add(m_prime_block[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        if i > 33
            && ((n_cond_nodes[RELATIVE_INDEX + i].tval ^ n_cond_nodes[RELATIVE_INDEX + i].val)
                != 0x80000000)
        {
            return i as i32;
        }
    }

    for i in 48..60 {
        n_cond_nodes[RELATIVE_INDEX + i].val = n_cond_nodes[RELATIVE_INDEX + i - 1]
            .val
            .overflowing_add(cls(
                md5_i(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                .0
                .overflowing_add(m_block[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        n_cond_nodes[RELATIVE_INDEX + i].tval = n_cond_nodes[RELATIVE_INDEX + i - 1]
            .tval
            .overflowing_add(cls(
                md5_i(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                .0
                .overflowing_add(m_prime_block[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        if n_cond_nodes[RELATIVE_INDEX + i]
            .tval
            .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + i].val)
            .0
            != dt[i]
        {
            return i as i32;
        }
    }

    for i in 60..64 {
        n_cond_nodes[RELATIVE_INDEX + i].val = n_cond_nodes[RELATIVE_INDEX + i - 1]
            .val
            .overflowing_add(cls(
                md5_i(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                .0
                .overflowing_add(m_block[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        n_cond_nodes[RELATIVE_INDEX + i].tval = n_cond_nodes[RELATIVE_INDEX + i - 1]
            .tval
            .overflowing_add(cls(
                md5_i(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                )
                .overflowing_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                .0
                .overflowing_add(m_prime_block[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;
    }

    n_cond_nodes[RELATIVE_INDEX + 68].val = n_cond_nodes[RELATIVE_INDEX + 60]
        .val
        .overflowing_add(n_cond_nodes[RELATIVE_INDEX - 4].val)
        .0;
    n_cond_nodes[RELATIVE_INDEX + 69].val = n_cond_nodes[RELATIVE_INDEX + 61]
        .val
        .overflowing_add(n_cond_nodes[RELATIVE_INDEX - 3].val)
        .0;
    n_cond_nodes[RELATIVE_INDEX + 70].val = n_cond_nodes[RELATIVE_INDEX + 62]
        .val
        .overflowing_add(n_cond_nodes[RELATIVE_INDEX - 2].val)
        .0;
    n_cond_nodes[RELATIVE_INDEX + 71].val = n_cond_nodes[RELATIVE_INDEX + 63]
        .val
        .overflowing_add(n_cond_nodes[RELATIVE_INDEX - 1].val)
        .0;
    n_cond_nodes[RELATIVE_INDEX + 68].tval = n_cond_nodes[RELATIVE_INDEX + 60]
        .tval
        .overflowing_add(n_cond_nodes[RELATIVE_INDEX - 4].val)
        .0;
    n_cond_nodes[RELATIVE_INDEX + 69].tval = n_cond_nodes[RELATIVE_INDEX + 61]
        .tval
        .overflowing_add(n_cond_nodes[RELATIVE_INDEX - 3].val)
        .0;
    n_cond_nodes[RELATIVE_INDEX + 70].tval = n_cond_nodes[RELATIVE_INDEX + 62]
        .tval
        .overflowing_add(n_cond_nodes[RELATIVE_INDEX - 2].val)
        .0;
    n_cond_nodes[RELATIVE_INDEX + 71].tval = n_cond_nodes[RELATIVE_INDEX + 63]
        .tval
        .overflowing_add(n_cond_nodes[RELATIVE_INDEX - 1].val)
        .0;

    for i in 69..72 {
        if fcheck_cond(i, n_cond_nodes) != 0 {
            return i;
        }
        if n_cond_nodes[RELATIVE_INDEX + i as usize]
            .tval
            .overflowing_sub(n_cond_nodes[RELATIVE_INDEX + i as usize].val)
            .0
            != dt[i as usize - 4]
        {
            return i as i32;
        }
    }

    return -1;
}

fn block1() -> ([u32; 4], [u32; 32], [u32; 32]) {
    let mut g_n19: u32 = 0;

    let mut rng = rand::thread_rng();

    // Building condition list and bitfield
    let mut re = build_condition_list("./data/md5cond_1.txt".to_string());
    build_bitfield(&mut re);
    let dt = construct_diff_table();
    // Initial random message
    let mut m_block: [u32; 32] = [0; 32];
    for i in 0..16 {
        m_block[i] = rng.gen();
    }
    first_block(&mut m_block, &mut re, dt, &mut g_n19);
    while check_diffs(&mut m_block, &mut re, 0, dt) > -1 {
        first_block(&mut m_block, &mut re, dt, &mut g_n19);
    }
    println!(
        "Block1ChainingValue: {:x}{:x}{:x}{:x}",
        re[RELATIVE_INDEX + 68].val,
        re[RELATIVE_INDEX + 71].val,
        re[RELATIVE_INDEX + 70].val,
        re[RELATIVE_INDEX + 69].val
    );
    let mut m_block_before: [u32; 32] = [0; 32];
    m_block_before.copy_from_slice(&m_block);

    m_block[4] = addsub_bit(m_block[4], 31, 1);
    m_block[11] = addsub_bit(m_block[11], 15, 1);
    m_block[14] = addsub_bit(m_block[14], 31, 1);

    return (
        [
            re[RELATIVE_INDEX + 68].val,
            re[RELATIVE_INDEX + 71].val,
            re[RELATIVE_INDEX + 70].val,
            re[RELATIVE_INDEX + 69].val,
        ],
        m_block_before,
        m_block,
    );
}

#[allow(unused_assignments)]
fn satisfy_stationary(q_cond_nodes: &mut [u32; 68], type1: i32, cond: [[u32; 3]; 309]) {
    let mut bit;
    let mut type_2;
    let k;
    let m_block;

    if type1 == 0 {
        m_block = 145;
        k = 211;
    } else if type1 == 2 {
        m_block = 0;
        k = 52;
    } else {
        m_block = 0;
        k = 274;
    }
    for mut i in m_block..k {
        let j = cond[i][0] + 4;
        let mut zero_bit: u32 = 0xffffffff;
        let mut one_bit: u32 = 0;
        while cond[i][0] == j - 4 {
            bit = cond[i][1];
            type_2 = cond[i][2];
            if type_2 == 0 {
                zero_bit = zero_bit & !(1 << (bit - 1));
            } else if type_2 == 1 {
                one_bit = one_bit | (1 << (bit - 1));
            } else if type_2 == 2 {
                if (q_cond_nodes[j as usize - 1] & (1 << (bit - 1))) != 0 {
                    one_bit = one_bit | (1 << (bit - 1));
                } else {
                    zero_bit = zero_bit & !(1 << (bit - 1));
                }
            } else if type_2 == 3 {
                if (q_cond_nodes[j as usize - 2] & (1 << (bit - 1))) != 0 {
                    one_bit = one_bit | (1 << (bit - 1));
                } else {
                    zero_bit = zero_bit & !(1 << (bit - 1));
                }
            } else if type_2 == 4 {
                if (q_cond_nodes[j as usize - 1] & (1 << (bit - 1))) == 0 {
                    one_bit = one_bit | (1 << (bit - 1));
                } else {
                    zero_bit = zero_bit & !(1 << (bit - 1));
                }
            }
            i += 1;
        }
        i -= 1;
        q_cond_nodes[j as usize] = q_cond_nodes[j as usize] | one_bit;
        q_cond_nodes[j as usize] = q_cond_nodes[j as usize] & zero_bit;
    }
}

#[inline]
fn md5_rr(var: u32, num: i32) -> u32 {
    let temp: u32 = var >> num;
    return (var << (32 - num)) | temp;
}
#[inline]
fn md5_rl(var: u32, num: i32) -> u32 {
    let temp: u32 = var << num;
    return (var >> (32 - num)) | temp;
}

fn findx(q_cond_nodes: &mut [u32; 68], m_block: &mut [u32; 16], m_prime_block: &mut [u32; 16]) {
    for i in 4..20 {
        m_block[i - 4] = md5_rr(
            q_cond_nodes[i].overflowing_sub(q_cond_nodes[i - 1]).0,
            SMAP[i - 4],
        )
        .overflowing_sub(TMAP[i - 4])
        .0
        .overflowing_sub(q_cond_nodes[i - 4])
        .0
        .overflowing_sub(phi(q_cond_nodes, i))
        .0;
        m_prime_block[i - 4] = m_block[i - 4];
    }
    m_prime_block[4] = m_prime_block[4].overflowing_sub(0x80000000).0;
    m_prime_block[11] = m_prime_block[11].overflowing_sub(0x8000).0;
    m_prime_block[14] = m_prime_block[14].overflowing_sub(0x80000000).0;
}

fn md5step20(
    m_block: &mut [u32; 16],
    vals: &mut [u32; 68],
    m_prime_block: &mut [u32; 16],
    vals1: &mut [u32; 68],
) {
    let mut a = vals[0];
    let mut b = vals[3];
    let mut c = vals[2];
    let mut d = vals[1];
    let mut t;
    let mut t1;
    for j in 0..16 {
        t = a
            .overflowing_add(
                ((b & c) | ((!b) & d))
                    .overflowing_add(m_block[MMAP[j] as usize])
                    .0
                    .overflowing_add(TMAP[j])
                    .0,
            )
            .0;
        let temp = d;
        d = c;
        c = b;
        a = temp;
        t1 = t >> (32 - SMAP[j]);
        b = b.overflowing_add((t << SMAP[j]).overflowing_add(t1).0).0;
        vals[j + 4] = b;
    }
    for j in 16..21 {
        t = a
            .overflowing_add((b & d) | (c & !d))
            .0
            .overflowing_add(m_block[MMAP[j] as usize])
            .0
            .overflowing_add(TMAP[j])
            .0;
        let temp = d;
        d = c;
        c = b;
        a = temp;
        t1 = t >> (32 - SMAP[j]);
        b = b.overflowing_add((t << SMAP[j]).overflowing_add(t1).0).0;
        vals[j + 4] = b;
    }
    a = vals1[0];
    b = vals1[3];
    c = vals1[2];
    d = vals1[1];
    // t;
    // t1;
    for j in 0..16 {
        t = a
            .overflowing_add((b & c) | ((!b) & d))
            .0
            .overflowing_add(m_prime_block[MMAP[j] as usize])
            .0
            .overflowing_add(TMAP[j])
            .0;
        let temp = d;
        d = c;
        c = b;
        a = temp;
        t1 = t >> (32 - SMAP[j]);
        b = b.overflowing_add((t << SMAP[j]) + t1).0;
        vals1[j + 4] = b;
    }
    for j in 16..21 {
        t = a
            .overflowing_add((b & d) | (c & !d))
            .0
            .overflowing_add(m_prime_block[MMAP[j] as usize])
            .0
            .overflowing_add(TMAP[j])
            .0;
        let temp = d;
        d = c;
        c = b;
        a = temp;
        t1 = t >> (32 - SMAP[j]);
        b = b.overflowing_add((t << SMAP[j]).overflowing_add(t1).0).0;
        vals1[j + 4] = b;
    }
}

#[allow(unused_assignments)]
fn check_stationary(q_cond_nodes: [u32; 68], m_block: i32, k: i32, cond: [[u32; 3]; 309]) -> bool {
    let mut bit;
    let mut type_2;
    for mut i in m_block..k {
        let j = cond[i as usize][0] + 4;
        let mut zero_bit: u32 = 0xffffffff;
        let mut one_bit: u32 = 0;
        while cond[i as usize][0] == j - 4 {
            bit = cond[i as usize][1];
            type_2 = cond[i as usize][2];
            if type_2 == 0 {
                zero_bit = zero_bit & !(1 << (bit - 1));
            } else if type_2 == 1 {
                one_bit = one_bit | (1 << (bit - 1));
            } else if type_2 == 2 {
                if (q_cond_nodes[j as usize - 1] & (1 << (bit - 1))) != 0 {
                    one_bit = one_bit | (1 << (bit - 1));
                } else {
                    zero_bit = zero_bit & !(1 << (bit - 1));
                }
            } else if type_2 == 3 {
                if (q_cond_nodes[j as usize - 2] & (1 << (bit - 1))) != 0 {
                    one_bit = one_bit | (1 << (bit - 1));
                } else {
                    zero_bit = zero_bit & !(1 << (bit - 1));
                }
            } else if type_2 == 4 {
                if (q_cond_nodes[j as usize - 1] & (1 << (bit - 1))) == 0 {
                    one_bit = one_bit | (1 << (bit - 1));
                } else {
                    zero_bit = zero_bit & !(1 << (bit - 1));
                }
            }
            i += 1;
        }
        i -= 1;
        if q_cond_nodes[j as usize] != (q_cond_nodes[j as usize] | one_bit) {
            return false;
        }
        if q_cond_nodes[j as usize] != (q_cond_nodes[j as usize] & zero_bit) {
            return false;
        }
    }
    return true;
}

fn block2(chaining_value: [u32; 4]) -> ([u32; 16], [u32; 16]) {
    let mut rng = rand::thread_rng();

    let mut q_cond_nodes: [u32; 68] = [0; 68];
    let mut q_prime: [u32; 68] = [0; 68];

    q_cond_nodes[0] = chaining_value[0];
    q_cond_nodes[1] = chaining_value[3];
    q_cond_nodes[2] = chaining_value[2];
    q_cond_nodes[3] = chaining_value[1];

    q_prime[0] = q_cond_nodes[0] ^ (0x80000000);
    q_prime[1] = q_cond_nodes[1] ^ (0x82000000);
    q_prime[2] = q_cond_nodes[2] ^ (0x86000000);
    q_prime[3] = q_cond_nodes[3] ^ (0x82000000);

    let cond: [[u32; 3]; 309] = build_condition_list_block_2("./data/md5cond_2.txt".to_string());

    let mut msg_found = false;
    while !msg_found {
        let mut c = true;

        let mut m_block: [u32; 16] = [0; 16];
        let mut m_prime_block: [u32; 16] = [0; 16];
        while c {
            let mut b = true;
            while b {
                for i in 4..20 {
                    q_cond_nodes[i] = rng.gen();
                }
                satisfy_stationary(&mut q_cond_nodes, 1, cond);
                findx(&mut q_cond_nodes, &mut m_block, &mut m_prime_block);
                if ((m_block[4] | m_block[14]) & 0x80000000) != 0 && (m_block[11] & 0x8000) != 0 {
                    md5step20(
                        &mut m_block,
                        &mut q_cond_nodes,
                        &mut m_prime_block,
                        &mut q_prime,
                    );
                    if (q_cond_nodes[19] ^ q_prime[19]) == 0xa0000000 {
                        if check_stationary(q_cond_nodes, 0, 274, cond) {
                            b = false;
                        }
                    }
                }
            }

            b = true;
            let mut number: i32 = 0;
            while b {
                number += 1;

                q_cond_nodes[5] = rng.gen();
                q_cond_nodes[4] = rng.gen();
                satisfy_stationary(&mut q_cond_nodes, 2, cond);
                findx(&mut q_cond_nodes, &mut m_block, &mut m_prime_block);
                md5step20(
                    &mut m_block,
                    &mut q_cond_nodes,
                    &mut m_prime_block,
                    &mut q_prime,
                );
                if number == 0x10000 {
                    b = false;
                }

                if ((q_cond_nodes[19] ^ q_prime[19]) == 0xa0000000)
                    && ((q_cond_nodes[24] ^ q_prime[24]) == 0x80000000)
                    && check_stationary(q_cond_nodes, 0, 286, cond)
                {
                    c = false;
                    b = false;
                }
            }
        }

        msg_found = multi_msg_2(
            &mut m_block,
            &mut m_prime_block,
            &mut q_cond_nodes,
            &mut q_prime,
        );
        if msg_found {
            println!(
                "Block2ChainingValue: {:x}{:x}{:x}{:x}",
                q_cond_nodes[64] + q_cond_nodes[0],
                q_cond_nodes[67] + q_cond_nodes[3],
                q_cond_nodes[66] + q_cond_nodes[2],
                q_cond_nodes[65] + q_cond_nodes[1]
            );

            return (m_block, m_prime_block);
        }
    }
    panic!("Block 2 failed");
}

fn md5step(
    m_block: &mut [u32; 16],
    out: &mut [u32; 68],
    m_prime_block: &mut [u32; 16],
    out1: &mut [u32; 68],
    j: usize,
) {
    let mut t;
    let mut t1;
    t = out[j]
        .overflowing_add(cover_func(out[j + 3], out[j + 2], out[j + 1], j))
        .0
        .overflowing_add(m_block[MMAP[j] as usize])
        .0
        .overflowing_add(TMAP[j])
        .0;
    t1 = t >> (32 - SMAP[j]);
    t1 = out[j + 3]
        .overflowing_add((t << SMAP[j]).overflowing_add(t1).0)
        .0;
    out[j + 4] = t1;
    t = out1[j]
        .overflowing_add(cover_func(out1[j + 3], out1[j + 2], out1[j + 1], j))
        .0
        .overflowing_add(m_prime_block[MMAP[j] as usize])
        .0
        .overflowing_add(TMAP[j])
        .0;
    t1 = t >> (32 - SMAP[j]);
    t1 = out1[j + 3]
        .overflowing_add((t << SMAP[j]).overflowing_add(t1).0)
        .0;
    out1[j + 4] = t1;
}

#[allow(unused_assignments)]
fn multi_msg_2(
    m_block: &mut [u32; 16],
    m_prime_block: &mut [u32; 16],
    q_cond_nodes: &mut [u32; 68],
    q_prime: &mut [u32; 68],
) -> bool {
    let mut rng = rand::thread_rng();
    for _ in 1..0x1000 {
        q_prime[19] = 0;
        while ((q_cond_nodes[24] ^ q_prime[24]) != 0x80000000)
            || ((q_cond_nodes[19] ^ q_prime[19]) != 0xa0000000)
        {
            q_cond_nodes[11] = ((rng.gen::<u32>()) & 0xe47efffe) | 0x843283c0;
            if (q_cond_nodes[10] & 0x2) == 0 {
                q_cond_nodes[11] = q_cond_nodes[11] & 0xfffffffd;
            } else {
                q_cond_nodes[11] = q_cond_nodes[11] | 0x2;
            }
            q_cond_nodes[12] = ((rng.gen::<u32>()) & 0xfc7d7dfd) | 0x9c0101c1;
            if (q_cond_nodes[11] & 0x1000) == 0 {
                q_cond_nodes[12] = q_cond_nodes[12] & 0xffffefff;
            } else {
                q_cond_nodes[12] = q_cond_nodes[12] | 0x1000;
            }
            q_cond_nodes[13] = ((rng.gen::<u32>()) & 0xfffbeffc) | 0x878383c0;
            q_cond_nodes[14] = ((rng.gen::<u32>()) & 0xfffdefff) | 0x800583c3;
            if (q_cond_nodes[13] & 0x80000) == 0 {
                q_cond_nodes[14] = q_cond_nodes[14] & 0xfff7ffff;
            } else {
                q_cond_nodes[14] = q_cond_nodes[14] | 0x80000;
            }
            if (q_cond_nodes[13] & 0x4000) == 0 {
                q_cond_nodes[14] = q_cond_nodes[14] & 0xffffbfff;
            } else {
                q_cond_nodes[14] = q_cond_nodes[14] | 0x4000;
            }
            if (q_cond_nodes[13] & 0x2000) == 0 {
                q_cond_nodes[14] = q_cond_nodes[14] & 0xffffdfff;
            } else {
                q_cond_nodes[14] = q_cond_nodes[14] | 0x2000;
            }
            if (q_cond_nodes[10] & 0x80000000) == 0 {
                q_cond_nodes[11] = q_cond_nodes[11] & 0x7fffffff;
                q_cond_nodes[12] = q_cond_nodes[12] & 0x7fffffff;
                q_cond_nodes[13] = q_cond_nodes[13] & 0x7fffffff;
                q_cond_nodes[14] = q_cond_nodes[14] & 0x7fffffff;
            }

            q_cond_nodes[15] = q_cond_nodes[14]
                .overflowing_add(md5_rl(
                    phi(q_cond_nodes, 15)
                        .overflowing_add(0x895cd7be)
                        .0
                        .overflowing_add(m_block[11])
                        .0
                        .overflowing_add(q_cond_nodes[11])
                        .0,
                    22,
                ))
                .0;

            if (q_cond_nodes[15] & 0xfff81fff) == q_cond_nodes[15]
                && (q_cond_nodes[15] | 0x00081080) == q_cond_nodes[15]
                && ((q_cond_nodes[14] ^ q_cond_nodes[15]) & 0xff000000) == 0
            {
                for i in 7..16 {
                    m_block[i] = md5_rr(
                        q_cond_nodes[i + 4].overflowing_sub(q_cond_nodes[i + 3]).0,
                        SMAP[i],
                    )
                    .overflowing_sub(TMAP[i])
                    .0
                    .overflowing_sub(q_cond_nodes[i])
                    .0
                    .overflowing_sub(phi(q_cond_nodes, i + 4))
                    .0;
                }
                for v in 7..16 {
                    m_prime_block[v] = m_block[v];
                }
                m_prime_block[11] = m_prime_block[11].overflowing_sub(0x8000).0;
                m_prime_block[14] = m_prime_block[14].overflowing_sub(0x80000000).0;
                md5step20(m_block, q_cond_nodes, m_prime_block, q_prime);
            }
        }

        for mut j in 0..0x20000 {
            let mut truth = true;
            if (j & 0x1) != 0 {
                if (q_cond_nodes[14] & 0x4) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x4;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x4;
                }
            } else if (j & 0x2) != 0 {
                if (q_cond_nodes[14] & 0x8) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x8;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x8;
                }
            } else if (j & 0x4) != 0 {
                if (q_cond_nodes[14] & 0x10) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x10;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x10;
                }
            } else if (j & 0x8) != 0 {
                if (q_cond_nodes[14] & 0x20) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x20;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x20;
                }
            } else if (j & 0x10) != 0 {
                if (q_cond_nodes[14] & 0x400) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x400;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x400;
                }
            } else if (j & 0x20) != 0 {
                if (q_cond_nodes[14] & 0x800) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x800;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x800;
                }
            } else if (j & 0x40) != 0 {
                if (q_cond_nodes[14] & 0x100000) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x100000;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x100000;
                }
            } else if (j & 0x80) != 0 {
                if (q_cond_nodes[14] & 0x200000) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x200000;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x200000;
                }
            } else if (j & 0x100) != 0 {
                if (q_cond_nodes[14] & 0x400000) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x400000;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x400000;
                }
            } else if (j & 0x200) != 0 {
                if (q_cond_nodes[14] & 0x20000000) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x20000000;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x20000000;
                }
            } else if (j & 0x400) != 0 {
                if (q_cond_nodes[14] & 0x40000000) == 0 {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x40000000;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x40000000;
                }
            } else if (j & 0x800) != 0 {
                if (q_cond_nodes[14] & 0x4000) == 0 {
                    j = j + 0x7ff;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x4000;
                }
            } else if (j & 0x1000) != 0 {
                if (q_cond_nodes[14] & 0x80000) == 0 {
                    j = j + 0xfff;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x80000;
                }
            } else if (j & 0x2000) != 0 {
                if (q_cond_nodes[14] & 0x40000) == 0 {
                    j = j + 0x1fff;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x40000;
                }
            } else if (j & 0x4000) != 0 {
                if (q_cond_nodes[14] & 0x8000000) != 0 {
                    j = j + 0x3fff;
                } else {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x8000000;
                }
            } else if (j & 0x8000) != 0 {
                if (q_cond_nodes[14] & 0x10000000) != 0 {
                    j = j + 0x7fff;
                } else {
                    q_cond_nodes[13] = q_cond_nodes[13] ^ 0x10000000;
                }
            } else if (j & 0x10000) != 0 {
                if (q_cond_nodes[14] & 0x2000) == 0 {
                    j = j + 0xffff;
                } else {
                    q_cond_nodes[12] = q_cond_nodes[12] ^ 0x2000;
                }
            }

            for p in 8..14 {
                m_block[p] = md5_rr(
                    q_cond_nodes[p + 4].overflowing_sub(q_cond_nodes[p + 3]).0,
                    SMAP[p],
                )
                .overflowing_sub(TMAP[p])
                .0
                .overflowing_sub(q_cond_nodes[p])
                .0
                .overflowing_sub(phi(q_cond_nodes, p + 4))
                .0;
                m_prime_block[p] = m_block[p];
            }
            m_prime_block[11] = m_prime_block[11] - 0x8000;
            md5step20(m_block, q_cond_nodes, m_prime_block, q_prime);
            for k in 21..64 {
                md5step(m_block, q_cond_nodes, m_prime_block, q_prime, k);
                if (q_cond_nodes[k + 4] ^ q_prime[k + 4]) != DIFFERENCES[k] {
                    truth = false;
                    break;
                }
            }
            if truth {
                let val64 = q_cond_nodes[64] + q_cond_nodes[0];
                let val65 = q_cond_nodes[65] + q_cond_nodes[1];
                let val66 = q_cond_nodes[66] + q_cond_nodes[2];
                let val67 = q_cond_nodes[67] + q_cond_nodes[3];
                let val164 = q_prime[64] + q_prime[0];
                let val165 = q_prime[65] + q_prime[1];
                let val166 = q_prime[66] + q_prime[2];
                let val167 = q_prime[67] + q_prime[3];

                if (val64 ^ val164) == 0
                    && (val65 ^ val165) == 0
                    && (val66 ^ val166) == 0
                    && (val67 ^ val167) == 0
                {
                    return true;
                }
            }
        }
    }
    return false;
}

fn main() {
    println!("---==[md5ium]==---");
    let mut cv_and_blocks1: ([u32; 4], [u32; 32], [u32; 32]) = block1();
    let blocks2: ([u32; 16], [u32; 16]) = block2(cv_and_blocks1.0);
    for i in 16..32 {
        cv_and_blocks1.1[i] = blocks2.0[i - 16];
        cv_and_blocks1.2[i] = blocks2.1[i - 16];
    }
    println!();
    println!("Block 1: {:x?}", cv_and_blocks1.1);
    println!("Block 2: {:x?}", cv_and_blocks1.2);

    // Converting the blocks to the proper format
    let mut output_1: Vec<u8> = Vec::new();
    let mut output_2: Vec<u8> = Vec::new();

    for hex in cv_and_blocks1.1 {
        let mut ii: [u8; 4] = hex.to_be_bytes();
        ii.reverse();
        for el in ii {
            output_1.push(el);
        }
    }
    for hex in cv_and_blocks1.2 {
        let mut ii: [u8; 4] = hex.to_be_bytes();
        ii.reverse();
        for el in ii {
            output_2.push(el);
        }
    }
    // Writing the blocks to a file
    let mut f1 = File::create("b1.bin").unwrap();
    f1.write_all(output_1.as_slice()).unwrap();
    let mut f2 = File::create("b2.bin").unwrap();
    f2.write_all(output_2.as_slice()).unwrap();
}
