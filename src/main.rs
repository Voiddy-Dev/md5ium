use rand::Rng;
use std::fs::File;
use std::io::{BufRead, BufReader};

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
    // println!("{}", return_val);
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
fn F(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (!x & z)
}
#[inline]
fn G(x: u32, y: u32, z: u32) -> u32 {
    return (x & z) | (y & (!z));
}
#[inline]
fn H(x: u32, y: u32, z: u32) -> u32 {
    return x ^ y ^ z;
}
#[inline]
fn I(x: u32, y: u32, z: u32) -> u32 {
    return y ^ (x | (!z));
}

#[inline]
fn cover_func(b: u32, c: u32, d: u32, i: usize) -> u32 {
    if i < 16 {
        return F(b, c, d);
    }
    if i < 32 {
        return G(b, c, d);
    }
    if i < 48 {
        return H(b, c, d);
    }
    return I(b, c, d);
}

#[inline]
fn phi(Q: &mut [u32; 68], i: usize) -> u32 {
    return cover_func(Q[i - 1], Q[i - 2], Q[i - 3], i - 4);
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
    Tval: u32,
    bf: [u32; 4],
    list: Vec<Condition>,
}

impl Default for Node {
    fn default() -> Node {
        Node {
            val: 0,
            Tval: 0,
            bf: [0, 0, 0, 0],
            list: Vec::new(),
        }
    }
}

fn smm5(index: i32, N: &mut Vec<Node>) -> u32 {
    let mut y: u32;
    let mut b2: i32; // might be u32????
    let mut i1: i32;
    let mut i2: i32;
    let mut i3: i32;
    let mut i4: i32;

    let mut x = N[RELATIVE_INDEX + index as usize].val;
    // println!("First x: {}", x);
    for el in &N[RELATIVE_INDEX + index as usize].list {
        // println!("\tlist {} ", el.cref);
        if el.cref < 0
        // condition of form a_i,j = 0/1
        {
            x = set_bit(x, el.ind, el.cref + 2);
            // println!("\tx --> {}", x);
        } else
        // condition of form a_i,j = b_k,l
        {
            y = N[RELATIVE_INDEX + el.cref as usize].val;
            b2 = get_bit(y, el.crind) as i32;
            x = set_bit(x, el.ind, b2);
            // println!("\tx2 --> {}", x);
        }
    }
    N[RELATIVE_INDEX + index as usize].val = x;
    // println!("X: {} - index: {}", x, RELATIVE_INDEX + index as usize);
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
    // println!("{} {} {} {}", i1, i2, i3, i4);
    // recompute correct message value for updated value of x
    return crs(
        x.overflowing_sub(N[RELATIVE_INDEX + i1 as usize].val).0,
        SMAP[index as usize],
    )
    .overflowing_sub(N[RELATIVE_INDEX + i4 as usize].val)
    .0
    .overflowing_sub(F(
        N[RELATIVE_INDEX + i1 as usize].val,
        N[RELATIVE_INDEX + i2 as usize].val,
        N[RELATIVE_INDEX + i3 as usize].val,
    ))
    .0
    .overflowing_sub(TMAP[index as usize])
    .0;
}

fn build_bitfield(N: &mut Vec<Node>) {
    let mut count = 0;
    for el in N {
        if count >= RELATIVE_INDEX {
            let mut list_iter = el.list.iter();
            while let Some(li) = list_iter.next() {
                // if count == 20 {
                // println!("{} {} {} {}", li.ind, li.cref, li.crind, li.add_const);
                // }
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
                // println!("{} {} {} {}", el.bf[0], el.bf[1], el.bf[2], el.bf[3]);
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
    // diff_table[6] -= addsub_bit(0, 27, 1);
    diff_table[6] = diff_table[6].wrapping_sub(addsub_bit(0, 27, 1));
    diff_table[7] += 1;
    // diff_table[7] -= addsub_bit(0, 15, 1);
    // diff_table[7] -= addsub_bit(0, 17, 1);
    // diff_table[7] -= addsub_bit(0, 23, 1);
    diff_table[7] = diff_table[7].wrapping_sub(addsub_bit(0, 15, 1));
    diff_table[7] = diff_table[7].wrapping_sub(addsub_bit(0, 17, 1));
    diff_table[7] = diff_table[7].wrapping_sub(addsub_bit(0, 23, 1));
    diff_table[8] += 1;
    // diff_table[8] -= addsub_bit(0, 6, 1);
    diff_table[8] = diff_table[8].wrapping_sub(addsub_bit(0, 6, 1));
    // diff_table[8] += addsub_bit(0, 31, 1);
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

fn first_round(M: &mut [u32; 32], N: &mut Vec<Node>, diff_table: [u32; 68]) {
    let mut flag: i32 = 0;

    while flag == 0 {
        flag = 1;

        for i in 0..16 {
            // Do initial computation
            N[RELATIVE_INDEX + i].val = N[RELATIVE_INDEX + i - 1]
                .val
                .overflowing_add(cls(
                    F(
                        N[RELATIVE_INDEX + i - 1].val,
                        N[RELATIVE_INDEX + i - 2].val,
                        N[RELATIVE_INDEX + i - 3].val,
                    )
                    .overflowing_add(N[RELATIVE_INDEX + i - 4].val)
                    .0
                    .overflowing_add(M[i])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;
            // println!("{}", N[RELATIVE_INDEX + i].val );
            // perform single-message modifications
            M[i] = smm5(i as i32, N);
            // println!("{}",  M[i] );
            // re-comupte value from new message value
            N[RELATIVE_INDEX + i].val = N[RELATIVE_INDEX + i - 1]
                .val
                .overflowing_add(cls(
                    F(
                        N[RELATIVE_INDEX + i - 1].val,
                        N[RELATIVE_INDEX + i - 2].val,
                        N[RELATIVE_INDEX + i - 3].val,
                    )
                    .overflowing_add(N[RELATIVE_INDEX + i - 4].val)
                    .0
                    .overflowing_add(M[i])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;
            // println!("{}", N[RELATIVE_INDEX + i].val );
            // println!("");
        }
        // compute offsets to compute differentials
        M[4] = addsub_bit(M[4], 31, 1);
        M[11] = addsub_bit(M[11], 15, 1);
        M[14] = addsub_bit(M[14], 31, 1);

        for i in 0..16 {
            N[RELATIVE_INDEX + i].Tval = N[RELATIVE_INDEX + i - 1]
                .Tval
                .overflowing_add(cls(
                    F(
                        N[RELATIVE_INDEX + i - 1].Tval,
                        N[RELATIVE_INDEX + i - 2].Tval,
                        N[RELATIVE_INDEX + i - 3].Tval,
                    )
                    .overflowing_add(N[RELATIVE_INDEX + i - 4].Tval)
                    .0
                    .overflowing_add(M[i])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;
            // println!("\t{}", N[RELATIVE_INDEX + i].Tval);

            // If differential isn't satisfied...
            // this doesn't occur very often because the enhanced
            // conditions are *almost* sufficient, but sometimes it does
            if N[RELATIVE_INDEX + i]
                .Tval
                .overflowing_sub(N[RELATIVE_INDEX + i].val)
                .0
                != diff_table[i]
            {
                flag = 0;
                new_randM(M);
                // panic!("BITCH IT RANDOMIZED");
            }
        }
        M[4] = addsub_bit(M[4], 31, -1);
        M[11] = addsub_bit(M[11], 15, -1);
        M[14] = addsub_bit(M[14], 31, -1);
        // println!("{} {} {}", M[4], M[11], M[14]);
        // panic!();
    }
    // println!("Done with first round");
}

fn new_randM(M: &mut [u32; 32]) {
    let mut temp: [u32; 32] = [0; 32];
    temp.copy_from_slice(M);
    let mut rng = rand::thread_rng();
    for i in 0..16 {
        M[i] = rng.gen();
    }
    assert_ne!(&mut temp, M);
}

fn fcheck_cond(ind: i32, N: &mut Vec<Node>) -> u32 {
    let mut x: u32 = 0;
    x |= (!N[RELATIVE_INDEX + ind as usize].val) & N[RELATIVE_INDEX + ind as usize].bf[0];
    // println!(
    //     "{} {} {} {} {} {}",
    //     x,
    //     N[RELATIVE_INDEX + ind as usize].val,
    //     N[RELATIVE_INDEX + ind as usize].bf[0],
    //     N[RELATIVE_INDEX + ind as usize].bf[1],
    //     N[RELATIVE_INDEX + ind as usize].bf[2],
    //     N[RELATIVE_INDEX + ind as usize].bf[3]
    // );
    x |= N[RELATIVE_INDEX + ind as usize].val & N[RELATIVE_INDEX + ind as usize].bf[1];
    // println!("{} {} ", x, N[RELATIVE_INDEX + ind as usize - 1].val);
    x |= (N[RELATIVE_INDEX + ind as usize - 1].val & N[RELATIVE_INDEX + ind as usize].bf[2])
        ^ (N[RELATIVE_INDEX + ind as usize].val & N[RELATIVE_INDEX + ind as usize].bf[2]);
    // println!("{} {}", x, N[RELATIVE_INDEX + ind as usize].bf[3]);
    if N[RELATIVE_INDEX + ind as usize].bf[3] != 0 {
        let list_iter = N[RELATIVE_INDEX + ind as usize].list.iter();
        let li = list_iter.last();
        match li {
            Some(list) => {
                // println!("YEAH RIGHT HERE {:?}", list);
                x |= (!(N[list.crind as usize].val) & N[RELATIVE_INDEX + ind as usize].bf[3])
                    ^ (N[RELATIVE_INDEX + ind as usize].val
                        & N[RELATIVE_INDEX + ind as usize].bf[2]);
            }
            _ => {
                panic!("BRUV LI MUST BE SOME");
            }
        }
    }
    x
}

fn klima1_3(M: &mut [u32; 32], N: &mut Vec<Node>) -> bool {
    let mut rng = rand::thread_rng();
    let mut x: u32;

    // println!("AT KLIMA 1_3");

    N[RELATIVE_INDEX + 17].val = 0;
    let mut count = 0;
    // println!("Check cond {} {}",  check_cond(18, N), check_cond(18, N) != 0); //, fcheck_cond(18, N));
    while (fcheck_cond(17, N) != 0) || (fcheck_cond(18, N) != 0) {
        count += 1;
        if count > 4096 {
            return true;
        }

        N[RELATIVE_INDEX + 16].val = rng.gen();
        x = N[RELATIVE_INDEX + 16].val;
        for (_, list) in N[RELATIVE_INDEX + 16 as usize].list.iter().enumerate() {
            if list.cref < 0 {
                x = set_bit(x, list.ind, list.cref + 2);
            } else {
                x = set_bit(
                    x,
                    list.ind,
                    get_bit(N[RELATIVE_INDEX + list.cref as usize].val, list.crind) as i32,
                );
            }
        }

        N[RELATIVE_INDEX + 16].val = x;
        N[RELATIVE_INDEX + 17].val = N[RELATIVE_INDEX + 16]
            .val
            .overflowing_add(cls(
                G(
                    N[RELATIVE_INDEX + 16].val,
                    N[RELATIVE_INDEX + 15].val,
                    N[RELATIVE_INDEX + 14].val,
                )
                .overflowing_add(N[RELATIVE_INDEX + 13].val)
                .0
                .overflowing_add(M[6])
                .0
                .overflowing_add(TMAP[17])
                .0,
                SMAP[17],
            ))
            .0;
        N[RELATIVE_INDEX + 18].val = N[RELATIVE_INDEX + 17]
            .val
            .overflowing_add(cls(
                G(
                    N[RELATIVE_INDEX + 17].val,
                    N[RELATIVE_INDEX + 16].val,
                    N[RELATIVE_INDEX + 15].val,
                )
                .overflowing_add(N[RELATIVE_INDEX + 14].val)
                .0
                .overflowing_add(M[11])
                .0
                .overflowing_add(TMAP[18])
                .0,
                SMAP[18],
            ))
            .0;
        // println!("Count {}", count);
    }
    false
}

fn klima4_9(M: &mut [u32; 32], N: &mut Vec<Node>, g_n19: &mut u32) {
    N[RELATIVE_INDEX + 19].val = *g_n19;
    *g_n19 += 1;
    fix_n19(g_n19);
    // println!("g_n19 {}", g_n19);
    // fix this value to satisfy the conditions (one for Klima, a couple
    // more for my modifications)
    // compute M_0 as in step 5 of Klima paper
    M[0] = crs(
        N[RELATIVE_INDEX + 19]
            .val
            .overflowing_sub(N[RELATIVE_INDEX + 18].val)
            .0,
        20,
    )
    .overflowing_sub(G(
        N[RELATIVE_INDEX + 18].val,
        N[RELATIVE_INDEX + 17].val,
        N[RELATIVE_INDEX + 16].val,
    ))
    .0
    .overflowing_sub(N[RELATIVE_INDEX + 15].val)
    .0
    .overflowing_sub(0xe9b6c7aa)
    .0;
    // compute N[0].val using step function
    N[RELATIVE_INDEX + 0].val = N[RELATIVE_INDEX + 67]
        .val
        .overflowing_add(cls(
            M[0].overflowing_add(N[RELATIVE_INDEX + 64].val)
                .0
                .overflowing_add(F(
                    N[RELATIVE_INDEX + 67].val,
                    N[RELATIVE_INDEX + 66].val,
                    N[RELATIVE_INDEX + 65].val,
                ))
                .0
                .overflowing_add(TMAP[0])
                .0,
            SMAP[0],
        ))
        .0;
    // compute M_1 as in step 3 of Klima paper
    M[1] = crs(
        N[RELATIVE_INDEX + 16]
            .val
            .overflowing_sub(N[RELATIVE_INDEX + 15].val)
            .0,
        5,
    )
    .overflowing_sub(G(
        N[RELATIVE_INDEX + 15].val,
        N[RELATIVE_INDEX + 14].val,
        N[RELATIVE_INDEX + 13].val,
    ))
    .0
    .overflowing_sub(N[RELATIVE_INDEX + 12].val)
    .0
    .overflowing_sub(0xf61e2562)
    .0;
    // compute N[1].val using step function
    N[RELATIVE_INDEX + 1].val = N[RELATIVE_INDEX + 0]
        .val
        .overflowing_add(cls(
            M[1].overflowing_add(N[RELATIVE_INDEX + 65].val)
                .0
                .overflowing_add(F(
                    N[RELATIVE_INDEX + 0].val,
                    N[RELATIVE_INDEX + 67].val,
                    N[RELATIVE_INDEX + 66].val,
                ))
                .0
                .overflowing_add(TMAP[1])
                .0,
            SMAP[1],
        ))
        .0;
    // compute M_2 as in step 3 of Klima paper
    M[2] = crs(
        N[RELATIVE_INDEX + 2]
            .val
            .overflowing_sub(N[RELATIVE_INDEX + 1].val)
            .0,
        17,
    )
    .overflowing_sub(F(
        N[RELATIVE_INDEX + 1].val,
        N[RELATIVE_INDEX + 0].val,
        N[RELATIVE_INDEX + 67].val,
    ))
    .0
    .overflowing_sub(N[RELATIVE_INDEX + 66].val)
    .0
    .overflowing_sub(TMAP[2])
    .0;
    // compute M_3 as in step 3 of Klima paper
    M[3] = crs(
        N[RELATIVE_INDEX + 3]
            .val
            .overflowing_sub(N[RELATIVE_INDEX + 2].val)
            .0,
        22,
    )
    .overflowing_sub(F(
        N[RELATIVE_INDEX + 2].val,
        N[RELATIVE_INDEX + 1].val,
        N[RELATIVE_INDEX + 0].val,
    ))
    .0
    .overflowing_sub(N[RELATIVE_INDEX + 67].val)
    .0
    .overflowing_sub(TMAP[3])
    .0;
    // compute M_4 as in step 3 of Klima paper
    M[4] = crs(
        N[RELATIVE_INDEX + 4]
            .val
            .overflowing_sub(N[RELATIVE_INDEX + 3].val)
            .0,
        7,
    )
    .overflowing_sub(F(
        N[RELATIVE_INDEX + 3].val,
        N[RELATIVE_INDEX + 2].val,
        N[RELATIVE_INDEX + 1].val,
    ))
    .0
    .overflowing_sub(N[RELATIVE_INDEX + 0].val)
    .0
    .overflowing_sub(TMAP[4])
    .0;
    // compute M_5 as in step 3 of Klima paper
    M[5] = crs(
        N[RELATIVE_INDEX + 5]
            .val
            .overflowing_sub(N[RELATIVE_INDEX + 4].val)
            .0,
        12,
    )
    .overflowing_sub(F(
        N[RELATIVE_INDEX + 4].val,
        N[RELATIVE_INDEX + 3].val,
        N[RELATIVE_INDEX + 2].val,
    ))
    .0
    .overflowing_sub(N[RELATIVE_INDEX + 1].val)
    .0
    .overflowing_sub(TMAP[5])
    .0;
    N[RELATIVE_INDEX + 20].val = N[RELATIVE_INDEX + 19]
        .val
        .overflowing_add(cls(
            M[5].overflowing_add(N[RELATIVE_INDEX + 16].val)
                .0
                .overflowing_add(G(
                    N[RELATIVE_INDEX + 19].val,
                    N[RELATIVE_INDEX + 18].val,
                    N[RELATIVE_INDEX + 17].val,
                ))
                .0
                .overflowing_add(0xd62f105d)
                .0,
            5,
        ))
        .0;
    // println!(
    //     "{} {} {} {} {}",
    //     M[3],
    //     N[RELATIVE_INDEX + 1].val,
    //     M[4],
    //     N[RELATIVE_INDEX + 20].val,
    //     M[5]
    // );
    if fcheck_cond(20, N) != 0 {
        *g_n19 += 0x7f;
        fix_n19(g_n19);
        N[RELATIVE_INDEX + 19].val = *g_n19;
        // println!("{} {}", N[RELATIVE_INDEX + 19].val, *g_n19);
    }
    // println!(" ========= end klima 4_9 ========= ");
}

fn first_block(M: &mut [u32; 32], N: &mut Vec<Node>, dt: [u32; 68], g_n19: &mut u32) {
    // Store IV in appropriate data structures
    N[RELATIVE_INDEX + 64].val = IV[0];
    N[RELATIVE_INDEX - 4].val = IV[0];
    N[RELATIVE_INDEX - 4].Tval = IV[0];

    N[RELATIVE_INDEX + 65].val = IV[3];
    N[RELATIVE_INDEX - 3].val = IV[3];
    N[RELATIVE_INDEX - 3].Tval = IV[3];

    N[RELATIVE_INDEX + 66].val = IV[2];
    N[RELATIVE_INDEX - 2].val = IV[2];
    N[RELATIVE_INDEX - 2].Tval = IV[2];

    N[RELATIVE_INDEX + 67].val = IV[1];
    N[RELATIVE_INDEX - 1].val = IV[1];
    N[RELATIVE_INDEX - 1].Tval = IV[1];

    // find message such that all first round conditions and differentials
    // are satisfied - this should be fast
    // for i in 0..72 {
    //     println!("{} {}", N[RELATIVE_INDEX + i].val, N[RELATIVE_INDEX + i].Tval);
    // }
    // for i in 0..32{
    //     println!("i M {}", M[i]);
    // }
    // println!("FIRST ROUND");
    first_round(M, N, dt);
    // for i in 0..72 {
    //     println!("{} {}", N[RELATIVE_INDEX + i].val, N[RELATIVE_INDEX + i].Tval);
    // }
    // for i in 0..32{
    //     println!("i M {}", M[i]);
    // }
    // klima1_3(M, N);
    // klima4_9(M, N, g_n19);
    // println!("DONE WITH KLIMA 4_9");
    // for i in 0..72 {
    //     println!(
    //         "N[{}]: {} {}",
    //         i,
    //         N[RELATIVE_INDEX + i].val,
    //         N[RELATIVE_INDEX + i].Tval
    //     );
    // }
    // for i in 0..32 {
    //     println!("i M {}", M[i]);
    // }
    // panic!();
    // do the first setup steps from Klima's code (steps 1-3)
    while klima1_3(M, N)
    // sometimes klima1_3 cannot be completed for
    {
        // certain values of Q_{0-15}
        new_randM(M);
        first_round(M, N, dt);
    }

    // // iterating over possible values for N[19], check to see if all
    // // other differentials/conditions hold
    klima4_9(M, N, g_n19);
    let mut stepno = check_diffs(M, N, 20, dt);
    // println!("Stepno {}", stepno);
    // panic!();

    while stepno >= 0 {
        if *g_n19 >= 0x80000000 {
            // println!("\tG TOO MUCH {}", stepno);
            *g_n19 = 0;
            while klima1_3(M, N) {
                new_randM(M);
                first_round(M, N, dt);
            }
        }
        // iterate over values of N[19]
        klima4_9(M, N, g_n19);
        stepno = check_diffs(M, N, 20, dt);
        // println!("Stepno {} - g_n19 {}", stepno, g_n19);
    }
    // println!("BRUV SUCCESS - stepno {}", stepno);
}

fn check_diffs(M: &mut [u32; 32], N: &mut Vec<Node>, index: i32, dt: [u32; 68]) -> i32 {
    let mut Mprime: [u32; 16] = [0; 16];
    Mprime.copy_from_slice(&M[..16]);

    for i in 0..16 {
        assert_eq!(M[i], Mprime[i]);
    }

    Mprime[4] = addsub_bit(Mprime[4], 31, 1);
    Mprime[11] = addsub_bit(Mprime[11], 15, 1);
    Mprime[14] = addsub_bit(Mprime[14], 31, 1);
    let mut local_index: usize = index as usize;
    // println!("Local Index: {}", local_index);
    // for i in 0..16 {
    //     println!("Mprime {}", Mprime[i]);
    // }

    if local_index == 20 {
        for i in 15..20 {
            N[RELATIVE_INDEX + i].Tval = N[RELATIVE_INDEX + i].val.overflowing_add(dt[i]).0;
            // println!(
            //     "{} {}",
            //     N[RELATIVE_INDEX + i].Tval,
            //     N[RELATIVE_INDEX + i].val
            // );
        }
    }

    if local_index != 20 {
        for i in 0..16 {
            // println!(
            //     "{} {} {}",
            //     N[RELATIVE_INDEX + i].Tval,
            //     N[RELATIVE_INDEX + i].val,
            //     dt[i]
            // );

            N[RELATIVE_INDEX + i].val = N[RELATIVE_INDEX + i - 1]
                .val
                .overflowing_add(cls(
                    F(
                        N[RELATIVE_INDEX + i - 1].val,
                        N[RELATIVE_INDEX + i - 2].val,
                        N[RELATIVE_INDEX + i - 3].val,
                    )
                    .overflowing_add(N[RELATIVE_INDEX + i - 4].val)
                    .0
                    .overflowing_add(M[MMAP[i] as usize])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;
            // println!(
            //     "\t{} {} {}",
            //     N[RELATIVE_INDEX + i].Tval,
            //     N[RELATIVE_INDEX + i].val,
            //     dt[i]
            // );

            // println!(
            //     "\tCLS: {}",
            //     cls(
            //         F(
            //             N[RELATIVE_INDEX + i - 1].Tval,
            //             N[RELATIVE_INDEX + i - 2].Tval,
            //             N[RELATIVE_INDEX + i - 3].Tval,
            //         )
            //         .overflowing_add(N[RELATIVE_INDEX + i - 4].Tval)
            //         .0
            //         .overflowing_add(Mprime[MMAP[i] as usize])
            //         .0
            //         .overflowing_add(TMAP[i])
            //         .0,
            //         SMAP[i],
            //     )
            // );

            N[RELATIVE_INDEX + i].Tval = N[RELATIVE_INDEX + i - 1]
                .Tval
                .overflowing_add(cls(
                    F(
                        N[RELATIVE_INDEX + i - 1].Tval,
                        N[RELATIVE_INDEX + i - 2].Tval,
                        N[RELATIVE_INDEX + i - 3].Tval,
                    )
                    .overflowing_add(N[RELATIVE_INDEX + i - 4].Tval)
                    .0
                    .overflowing_add(Mprime[MMAP[i] as usize])
                    .0
                    .overflowing_add(TMAP[i])
                    .0,
                    SMAP[i],
                ))
                .0;

            // println!(
            //     "{} {} {}",
            //     N[RELATIVE_INDEX + i].Tval,
            //     N[RELATIVE_INDEX + i].val,
            //     dt[i]
            // );
            if N[RELATIVE_INDEX + i]
                .Tval
                .overflowing_sub(N[RELATIVE_INDEX + i].val)
                .0
                != dt[i]
            {
                return i as i32;
            }
        }
        local_index = 16;
    }

    for i in local_index..32 {
        N[RELATIVE_INDEX + i].val = N[RELATIVE_INDEX + i - 1]
            .val
            .overflowing_add(cls(
                G(
                    N[RELATIVE_INDEX + i - 1].val,
                    N[RELATIVE_INDEX + i - 2].val,
                    N[RELATIVE_INDEX + i - 3].val,
                )
                .overflowing_add(N[RELATIVE_INDEX + i - 4].val)
                .0
                .overflowing_add(M[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        N[RELATIVE_INDEX + i].Tval = N[RELATIVE_INDEX + i - 1]
            .Tval
            .overflowing_add(cls(
                G(
                    N[RELATIVE_INDEX + i - 1].Tval,
                    N[RELATIVE_INDEX + i - 2].Tval,
                    N[RELATIVE_INDEX + i - 3].Tval,
                )
                .overflowing_add(N[RELATIVE_INDEX + i - 4].Tval)
                .0
                .overflowing_add(Mprime[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;
        if N[RELATIVE_INDEX + i]
            .Tval
            .overflowing_sub(N[RELATIVE_INDEX + i].val)
            .0
            != dt[i]
        {
            return i as i32;
        }
    }

    for i in 32..48 {
        N[RELATIVE_INDEX + i].val = N[RELATIVE_INDEX + i - 1]
            .val
            .overflowing_add(cls(
                H(
                    N[RELATIVE_INDEX + i - 1].val,
                    N[RELATIVE_INDEX + i - 2].val,
                    N[RELATIVE_INDEX + i - 3].val,
                )
                .overflowing_add(N[RELATIVE_INDEX + i - 4].val)
                .0
                .overflowing_add(M[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        N[RELATIVE_INDEX + i].Tval = N[RELATIVE_INDEX + i - 1]
            .Tval
            .overflowing_add(cls(
                H(
                    N[RELATIVE_INDEX + i - 1].Tval,
                    N[RELATIVE_INDEX + i - 2].Tval,
                    N[RELATIVE_INDEX + i - 3].Tval,
                )
                .overflowing_add(N[RELATIVE_INDEX + i - 4].Tval)
                .0
                .overflowing_add(Mprime[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        if i > 33 && ((N[RELATIVE_INDEX + i].Tval ^ N[RELATIVE_INDEX + i].val) != 0x80000000) {
            return i as i32;
        }
    }

    for i in 48..60 {
        N[RELATIVE_INDEX + i].val = N[RELATIVE_INDEX + i - 1]
            .val
            .overflowing_add(cls(
                I(
                    N[RELATIVE_INDEX + i - 1].val,
                    N[RELATIVE_INDEX + i - 2].val,
                    N[RELATIVE_INDEX + i - 3].val,
                )
                .overflowing_add(N[RELATIVE_INDEX + i - 4].val)
                .0
                .overflowing_add(M[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        N[RELATIVE_INDEX + i].Tval = N[RELATIVE_INDEX + i - 1]
            .Tval
            .overflowing_add(cls(
                I(
                    N[RELATIVE_INDEX + i - 1].Tval,
                    N[RELATIVE_INDEX + i - 2].Tval,
                    N[RELATIVE_INDEX + i - 3].Tval,
                )
                .overflowing_add(N[RELATIVE_INDEX + i - 4].Tval)
                .0
                .overflowing_add(Mprime[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        if N[RELATIVE_INDEX + i]
            .Tval
            .overflowing_sub(N[RELATIVE_INDEX + i].val)
            .0
            != dt[i]
        {
            return i as i32;
        }
    }

    for i in 60..64 {
        N[RELATIVE_INDEX + i].val = N[RELATIVE_INDEX + i - 1]
            .val
            .overflowing_add(cls(
                I(
                    N[RELATIVE_INDEX + i - 1].val,
                    N[RELATIVE_INDEX + i - 2].val,
                    N[RELATIVE_INDEX + i - 3].val,
                )
                .overflowing_add(N[RELATIVE_INDEX + i - 4].val)
                .0
                .overflowing_add(M[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;

        N[RELATIVE_INDEX + i].Tval = N[RELATIVE_INDEX + i - 1]
            .Tval
            .overflowing_add(cls(
                I(
                    N[RELATIVE_INDEX + i - 1].Tval,
                    N[RELATIVE_INDEX + i - 2].Tval,
                    N[RELATIVE_INDEX + i - 3].Tval,
                )
                .overflowing_add(N[RELATIVE_INDEX + i - 4].Tval)
                .0
                .overflowing_add(Mprime[MMAP[i] as usize])
                .0
                .overflowing_add(TMAP[i])
                .0,
                SMAP[i],
            ))
            .0;
    }

    // Calculate new chaining variables
    N[RELATIVE_INDEX + 68].val = N[RELATIVE_INDEX + 60]
        .val
        .overflowing_add(N[RELATIVE_INDEX - 4].val)
        .0;
    N[RELATIVE_INDEX + 69].val = N[RELATIVE_INDEX + 61]
        .val
        .overflowing_add(N[RELATIVE_INDEX - 3].val)
        .0;
    N[RELATIVE_INDEX + 70].val = N[RELATIVE_INDEX + 62]
        .val
        .overflowing_add(N[RELATIVE_INDEX - 2].val)
        .0;
    N[RELATIVE_INDEX + 71].val = N[RELATIVE_INDEX + 63]
        .val
        .overflowing_add(N[RELATIVE_INDEX - 1].val)
        .0;
    N[RELATIVE_INDEX + 68].Tval = N[RELATIVE_INDEX + 60]
        .Tval
        .overflowing_add(N[RELATIVE_INDEX - 4].val)
        .0;
    N[RELATIVE_INDEX + 69].Tval = N[RELATIVE_INDEX + 61]
        .Tval
        .overflowing_add(N[RELATIVE_INDEX - 3].val)
        .0;
    N[RELATIVE_INDEX + 70].Tval = N[RELATIVE_INDEX + 62]
        .Tval
        .overflowing_add(N[RELATIVE_INDEX - 2].val)
        .0;
    N[RELATIVE_INDEX + 71].Tval = N[RELATIVE_INDEX + 63]
        .Tval
        .overflowing_add(N[RELATIVE_INDEX - 1].val)
        .0;

    for i in 69..72 {
        if fcheck_cond(i, N) != 0 {
            return i;
        }
        if N[RELATIVE_INDEX + i as usize]
            .Tval
            .overflowing_sub(N[RELATIVE_INDEX + i as usize].val)
            .0
            != dt[i as usize - 4]
        {
            return i as i32;
        }
    }

    return -1;
}

fn block1() -> [u32; 4] {
    let mut g_n19: u32 = 0;

    let mut rng = rand::thread_rng();

    // Building condition list and bitfield
    let mut re = build_condition_list("./data/md5cond_1.txt".to_string());
    build_bitfield(&mut re);
    let dt = construct_diff_table();
    // Initial random message
    let mut M: [u32; 32] = [0; 32];
    for i in 0..16 {
        M[i] = rng.gen();
    }
    first_block(&mut M, &mut re, dt, &mut g_n19);
    while check_diffs(&mut M, &mut re, 0, dt) > -1 {
        first_block(&mut M, &mut re, dt, &mut g_n19);
    }
    println!(
        "\n\tChaining value: {:x}{:x}{:x}{:x}",
        re[RELATIVE_INDEX + 68].val,
        re[RELATIVE_INDEX + 71].val,
        re[RELATIVE_INDEX + 70].val,
        re[RELATIVE_INDEX + 69].val
    );

    // Printing out message
    print!("M\t");
    for i in 0..15 {
        if i % 4 == 0 && i != 0 {
            print!("\n\t");
        }
        print!("{:x}, ", M[i]);
    }
    print!("{:x}\n\n", M[15]);
    M[4] = addsub_bit(M[4], 31, 1);
    M[11] = addsub_bit(M[11], 15, 1);
    M[14] = addsub_bit(M[14], 31, 1);
    print!("M'\t");
    for i in 0..15 {
        if i % 4 == 0 && i != 0 {
            print!("\n\t");
        }
        print!("{:x}, ", M[i]);
    }
    print!("{:x}\n\n", M[15]);

    return [
        re[RELATIVE_INDEX + 68].val,
        re[RELATIVE_INDEX + 71].val,
        re[RELATIVE_INDEX + 70].val,
        re[RELATIVE_INDEX + 69].val,
    ];
}

fn satisfy_stationary(Q: &mut [u32; 68], type1: i32, cond: [[u32; 3]; 309]) {
    let mut bit;
    let mut type_2;
    let mut j = 0;
    let mut k;
    let mut m;

    //satisfy Q[7-10] for multimessage
    if (type1 == 0) {
        m = 145;
        k = 211;
    }
    //satisfy Q[0,1]
    else if (type1 == 2) {
        m = 0;
        k = 52;
    }
    //satisfy Q[0-15]
    else {
        m = 0;
        k = 274;
    }
    //reads through conditions modifying Q[0-15] to satisfy their conditions
    for mut i in m..k {
        j = cond[i][0] + 4;
        let mut zeroBit: u32 = 0xffffffff;
        let mut oneBit: u32 = 0;
        while (cond[i][0] == j - 4) {
            bit = cond[i][1];
            type_2 = cond[i][2];
            //designated bit should be set to zero
            if (type_2 == 0) {
                zeroBit = zeroBit & !(1 << (bit - 1));
            }
            //designated bit should be set to one
            else if (type_2 == 1) {
                oneBit = oneBit | (1 << (bit - 1));
            }
            /*designated bit should be set eQual to the
            same bit of the previous chaining value*/
            else if (type_2 == 2) {
                if ((Q[j as usize - 1] & (1 << (bit - 1))) != 0) {
                    oneBit = oneBit | (1 << (bit - 1));
                } else {
                    zeroBit = zeroBit & !(1 << (bit - 1));
                }
            }
            /*designated bit in chaining value x should
            be set eQual to the same bit of chaining value x-2*/
            else if (type_2 == 3) {
                if ((Q[j as usize - 2] & (1 << (bit - 1))) != 0) {
                    oneBit = oneBit | (1 << (bit - 1));
                } else {
                    zeroBit = zeroBit & !(1 << (bit - 1));
                }
            }
            /*designated bit should be set to the negation
            of the same bit of the previous chaining value*/
            else if (type_2 == 4) {
                //printf("here");
                if ((Q[j as usize - 1] & (1 << (bit - 1))) == 0) {
                    oneBit = oneBit | (1 << (bit - 1));
                } else {
                    zeroBit = zeroBit & !(1 << (bit - 1));
                }
            }
            i += 1;
        }
        i -= 1;
        //modify Q[j] to satisfy conditions
        Q[j as usize] = Q[j as usize] | oneBit;
        Q[j as usize] = Q[j as usize] & zeroBit;
    }
}

#[inline]
fn RR(var: u32, num: i32) -> u32 {
    let temp: u32 = var >> num;
    return (var << (32 - num)) | temp;
}
#[inline]
fn RL(var: u32, num: i32) -> u32 {
    let temp: u32 = var << num;
    return (var >> (32 - num)) | temp;
}

fn findx(Q: &mut [u32; 68], M: &mut [u32; 16], Mprime: &mut [u32; 16]) {
    for i in 4..20 {
        M[i - 4] = RR((Q[i].overflowing_sub(Q[i - 1]).0), SMAP[i - 4])
            .overflowing_sub(TMAP[i - 4])
            .0
            .overflowing_sub(Q[i - 4])
            .0
            .overflowing_sub(phi(Q, i))
            .0;
        Mprime[i - 4] = M[i - 4];
    }
    Mprime[4] = Mprime[4].overflowing_sub(0x80000000).0;
    Mprime[11] = Mprime[11].overflowing_sub(0x8000).0;
    Mprime[14] = Mprime[14].overflowing_sub(0x80000000).0;
}

fn md5step20(
    M: &mut [u32; 16],
    vals: &mut [u32; 68],
    Mprime: &mut [u32; 16],
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
                    .overflowing_add(M[MMAP[j] as usize])
                    .0
                    .overflowing_add(TMAP[j])
                    .0,
            )
            .0;
        let mut temp = d;
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
            .overflowing_add(M[MMAP[j] as usize])
            .0
            .overflowing_add(TMAP[j])
            .0;
        let mut temp = d;
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
            .overflowing_add(Mprime[MMAP[j] as usize])
            .0
            .overflowing_add(TMAP[j])
            .0;
        let mut temp = d;
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
            .overflowing_add(Mprime[MMAP[j] as usize])
            .0
            .overflowing_add(TMAP[j])
            .0;
        let mut temp = d;
        d = c;
        c = b;
        a = temp;
        t1 = t >> (32 - SMAP[j]);
        b = b.overflowing_add((t << SMAP[j]).overflowing_add(t1).0).0;
        vals1[j + 4] = b;
    }
}

fn check_stationary(Q: [u32; 68], m: i32, k: i32, cond: [[u32; 3]; 309]) -> bool {
    let mut bit;
    let mut type_2;
    let mut j: u32 = 0;
    for mut i in m..k {
        j = cond[i as usize][0] + 4;
        let mut zeroBit: u32 = 0xffffffff;
        let mut oneBit: u32 = 0;
        while (cond[i as usize][0] == j - 4) {
            bit = cond[i as usize][1];
            type_2 = cond[i as usize][2];
            if (type_2 == 0) {
                zeroBit = zeroBit & !(1 << (bit - 1));
            } else if (type_2 == 1) {
                oneBit = oneBit | (1 << (bit - 1));
            } else if (type_2 == 2) {
                if ((Q[j as usize - 1] & (1 << (bit - 1))) != 0) {
                    oneBit = oneBit | (1 << (bit - 1));
                } else {
                    zeroBit = zeroBit & !(1 << (bit - 1));
                }
            } else if (type_2 == 3) {
                if ((Q[j as usize - 2] & (1 << (bit - 1))) != 0) {
                    oneBit = oneBit | (1 << (bit - 1));
                } else {
                    zeroBit = zeroBit & !(1 << (bit - 1));
                }
            } else if (type_2 == 4) {
                if ((Q[j as usize - 1] & (1 << (bit - 1))) == 0) {
                    oneBit = oneBit | (1 << (bit - 1));
                } else {
                    zeroBit = zeroBit & !(1 << (bit - 1));
                }
            }
            i += 1;
        }
        i -= 1;
        if (Q[j as usize] != (Q[j as usize] | oneBit)) {
            //printf("%d %x 1\n", j, Q[j]);
            return false;
        }
        if (Q[j as usize] != (Q[j as usize] & zeroBit)) {
            //printf("%d %x 2\n", j, Q[j]);
            return false;
        }
    }
    return true;
}

fn block2(CV: [u32; 4]) {
    let mut rng = rand::thread_rng();

    println!(
        "ChainingValue: {:x}{:x}{:x}{:x}",
        CV[0], CV[1], CV[2], CV[3]
    );

    let mut Q: [u32; 68] = [0; 68];
    let mut Qprime: [u32; 68] = [0; 68];

    Q[0] = CV[0];
    Q[1] = CV[3];
    Q[2] = CV[2];
    Q[3] = CV[1];

    Qprime[0] = Q[0] ^ (0x80000000);
    Qprime[1] = Q[1] ^ (0x82000000);
    Qprime[2] = Q[2] ^ (0x86000000);
    Qprime[3] = Q[3] ^ (0x82000000);

    // println!("Qprime: {:?}", Qprime);

    let cond: [[u32; 3]; 309] = build_condition_list_block_2("./data/md5cond_2.txt".to_string());
    // satisfy_stationary(&mut Q,1, cond);
    // let mut M: [u32; 16] = [0; 16];
    // let mut Mprime: [u32; 16] = [0; 16];
    // findx(&mut Q, &mut M, &mut Mprime);
    // md5step20(&mut M, &mut Q, &mut Mprime, &mut Qprime);
    // println!("M: {:?}", Q);
    // println!("M': {:?}", Qprime);

    let mut messageFound = false;
    while !messageFound {
        let mut b = true;
        let mut c = true;

        let mut M: [u32; 16] = [0; 16];
        let mut Mprime: [u32; 16] = [0; 16];
        while c {
            b = true;
            while b {
                for i in 4..20 {
                    Q[i] = rng.gen();
                }
                satisfy_stationary(&mut Q, 1, cond);
                findx(&mut Q, &mut M, &mut Mprime);
                if ((M[4] | M[14]) & 0x80000000) != 0 && (M[11] & 0x8000) != 0 {
                    md5step20(&mut M, &mut Q, &mut Mprime, &mut Qprime);
                    if (Q[19] ^ Qprime[19]) == 0xa0000000 {
                        if check_stationary(Q, 0, 274, cond) {
                            b = false;
                        }
                    }
                }
            }

            b = true;
            let mut number: i32 = 0;
            while b {
                number += 1;

                Q[5] = rng.gen();
                Q[4] = rng.gen();
                satisfy_stationary(&mut Q, 2, cond);
                findx(&mut Q, &mut M, &mut Mprime);
                md5step20(&mut M, &mut Q, &mut Mprime, &mut Qprime);
                if number == 0x10000 {
                    b = false;
                }

                if ((Q[19] ^ Qprime[19]) == 0xa0000000)
                    && ((Q[24] ^ Qprime[24]) == 0x80000000)
                    && check_stationary(Q, 0, 286, cond)
                {
                    c = false;
                    b = false;
                }
            }
        }

        messageFound = multiMessage2(&mut M, &mut Mprime, &mut Q, &mut Qprime);
        if messageFound {
            println!(
                "Block2ChainingValue: {:x}{:x}{:x}{:x}",
                Q[64] + Q[0],
                Q[67] + Q[3],
                Q[66] + Q[2],
                Q[65] + Q[1]
            );
            print!("M\t");
            for i in 0..15 {
                if i % 4 == 0 && i != 0 {
                    print!("\n\t");
                }
                print!("{:x}, ", M[i]);
            }
            print!("{:x}\n\n", M[15]);
            print!("M'\t");
            for i in 0..15 {
                if i % 4 == 0 && i != 0 {
                    print!("\n\t");
                }
                print!("{:x}, ", Mprime[i]);
            }
            print!("{:x}\n\n", Mprime[15]);
        }
    }
}

fn md5step(
    M: &mut [u32; 16],
    out: &mut [u32; 68],
    Mprime: &mut [u32; 16],
    out1: &mut [u32; 68],
    j: usize,
) {
    let mut t;
    let mut t1;
    t = out[j]
        .overflowing_add(cover_func(out[j + 3], out[j + 2], out[j + 1], j))
        .0
        .overflowing_add(M[MMAP[j] as usize])
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
        .overflowing_add(Mprime[MMAP[j] as usize])
        .0
        .overflowing_add(TMAP[j])
        .0;
    t1 = t >> (32 - SMAP[j]);
    t1 = out1[j + 3]
        .overflowing_add((t << SMAP[j]).overflowing_add(t1).0)
        .0;
    out1[j + 4] = t1;
}

fn multiMessage2(
    M: &mut [u32; 16],
    Mprime: &mut [u32; 16],
    Q: &mut [u32; 68],
    Qprime: &mut [u32; 68],
) -> bool {
    let mut rng = rand::thread_rng();
    for i in 1..0x1000 {
        Qprime[19] = 0;
        while ((Q[24] ^ Qprime[24]) != 0x80000000) || ((Q[19] ^ Qprime[19]) != 0xa0000000) {
            //randomly select Q[7-10] and satisfy conditons
            Q[11] = ((rng.gen::<u32>()) & 0xe47efffe) | 0x843283c0;
            //sets Q[7]_2 = Q[6]_2
            if ((Q[10] & 0x2) == 0) {
                Q[11] = Q[11] & 0xfffffffd;
            } else {
                Q[11] = Q[11] | 0x2;
            }
            Q[12] = ((rng.gen::<u32>()) & 0xfc7d7dfd) | 0x9c0101c1;
            if ((Q[11] & 0x1000) == 0) {
                Q[12] = Q[12] & 0xffffefff;
            } else {
                Q[12] = Q[12] | 0x1000;
            }
            Q[13] = ((rng.gen::<u32>()) & 0xfffbeffc) | 0x878383c0;
            Q[14] = ((rng.gen::<u32>()) & 0xfffdefff) | 0x800583c3;
            if ((Q[13] & 0x80000) == 0) {
                Q[14] = Q[14] & 0xfff7ffff;
            } else {
                Q[14] = Q[14] | 0x80000;
            }
            if ((Q[13] & 0x4000) == 0) {
                Q[14] = Q[14] & 0xffffbfff;
            } else {
                Q[14] = Q[14] | 0x4000;
            }
            if ((Q[13] & 0x2000) == 0) {
                Q[14] = Q[14] & 0xffffdfff;
            } else {
                Q[14] = Q[14] | 0x2000;
            }
            if ((Q[10] & 0x80000000) == 0) {
                Q[11] = Q[11] & 0x7fffffff;
                Q[12] = Q[12] & 0x7fffffff;
                Q[13] = Q[13] & 0x7fffffff;
                Q[14] = Q[14] & 0x7fffffff;
            }

            //calculate Q[11]
            Q[15] = Q[14]
                .overflowing_add(RL(
                    phi(Q, 15)
                        .overflowing_add(0x895cd7be)
                        .0
                        .overflowing_add(M[11])
                        .0
                        .overflowing_add(Q[11])
                        .0,
                    22,
                ))
                .0;

            if (Q[15] & 0xfff81fff) == Q[15]
                && (Q[15] | 0x00081080) == Q[15]
                && ((Q[14] ^ Q[15]) & 0xff000000) == 0
            {
                for i in 7..16 {
                    M[i] = RR(Q[i + 4].overflowing_sub(Q[i + 3]).0, SMAP[i])
                        .overflowing_sub(TMAP[i])
                        .0
                        .overflowing_sub(Q[i])
                        .0
                        .overflowing_sub(phi(Q, i + 4))
                        .0;
                }
                for v in 7..16 {
                    Mprime[v] = M[v];
                }
                Mprime[11] = Mprime[11].overflowing_sub(0x8000).0;
                Mprime[14] = Mprime[14].overflowing_sub(0x80000000).0;
                md5step20(M, Q, Mprime, Qprime);
            }
        }

        let mut truth = true;
        let mut x11 = Q[15];
        for mut j in 0..0x20000 {
            truth = true;
            //flip bits using gray code
            if ((j & 0x1) != 0) {
                if ((Q[14] & 0x4) == 0) {
                    Q[13] = Q[13] ^ 0x4;
                } else {
                    Q[12] = Q[12] ^ 0x4;
                }
            } else if ((j & 0x2) != 0) {
                if ((Q[14] & 0x8) == 0) {
                    Q[13] = Q[13] ^ 0x8;
                } else {
                    Q[12] = Q[12] ^ 0x8;
                }
            } else if ((j & 0x4) != 0) {
                if ((Q[14] & 0x10) == 0) {
                    Q[13] = Q[13] ^ 0x10;
                } else {
                    Q[12] = Q[12] ^ 0x10;
                }
            } else if ((j & 0x8) != 0) {
                if ((Q[14] & 0x20) == 0) {
                    Q[13] = Q[13] ^ 0x20;
                } else {
                    Q[12] = Q[12] ^ 0x20;
                }
            } else if ((j & 0x10) != 0) {
                if ((Q[14] & 0x400) == 0) {
                    Q[13] = Q[13] ^ 0x400;
                } else {
                    Q[12] = Q[12] ^ 0x400;
                }
            } else if ((j & 0x20) != 0) {
                if ((Q[14] & 0x800) == 0) {
                    Q[13] = Q[13] ^ 0x800;
                } else {
                    Q[12] = Q[12] ^ 0x800;
                }
            } else if ((j & 0x40) != 0) {
                if ((Q[14] & 0x100000) == 0) {
                    Q[13] = Q[13] ^ 0x100000;
                } else {
                    Q[12] = Q[12] ^ 0x100000;
                }
            } else if ((j & 0x80) != 0) {
                if ((Q[14] & 0x200000) == 0) {
                    Q[13] = Q[13] ^ 0x200000;
                } else {
                    Q[12] = Q[12] ^ 0x200000;
                }
            } else if ((j & 0x100) != 0) {
                if ((Q[14] & 0x400000) == 0) {
                    Q[13] = Q[13] ^ 0x400000;
                } else {
                    Q[12] = Q[12] ^ 0x400000;
                }
            } else if ((j & 0x200) != 0) {
                if ((Q[14] & 0x20000000) == 0) {
                    Q[13] = Q[13] ^ 0x20000000;
                } else {
                    Q[12] = Q[12] ^ 0x20000000;
                }
            } else if ((j & 0x400) != 0) {
                if ((Q[14] & 0x40000000) == 0) {
                    Q[13] = Q[13] ^ 0x40000000;
                } else {
                    Q[12] = Q[12] ^ 0x40000000;
                }
            } else if ((j & 0x800) != 0) {
                if ((Q[14] & 0x4000) == 0) {
                    j = j + 0x7ff;
                } else {
                    Q[12] = Q[12] ^ 0x4000;
                }
            } else if ((j & 0x1000) != 0) {
                if ((Q[14] & 0x80000) == 0) {
                    j = j + 0xfff;
                } else {
                    Q[12] = Q[12] ^ 0x80000;
                }
            } else if ((j & 0x2000) != 0) {
                if ((Q[14] & 0x40000) == 0) {
                    j = j + 0x1fff;
                } else {
                    Q[12] = Q[12] ^ 0x40000;
                }
            } else if ((j & 0x4000) != 0) {
                if ((Q[14] & 0x8000000) != 0) {
                    j = j + 0x3fff;
                } else {
                    Q[13] = Q[13] ^ 0x8000000;
                }
            } else if ((j & 0x8000) != 0) {
                if ((Q[14] & 0x10000000) != 0) {
                    j = j + 0x7fff;
                } else {
                    Q[13] = Q[13] ^ 0x10000000;
                }
            } else if ((j & 0x10000) != 0) {
                if ((Q[14] & 0x2000) == 0) {
                    j = j + 0xffff;
                } else {
                    Q[12] = Q[12] ^ 0x2000;
                }
            }

            for p in 8..14 {
                M[p] = RR(Q[p + 4].overflowing_sub(Q[p + 3]).0, SMAP[p])
                    .overflowing_sub(TMAP[p])
                    .0
                    .overflowing_sub(Q[p])
                    .0
                    .overflowing_sub(phi(Q, p + 4))
                    .0;
                Mprime[p] = M[p];
            }
            Mprime[11] = Mprime[11] - 0x8000;
            md5step20(M, Q, Mprime, Qprime);
            for k in 21..64 {
                md5step(M, Q, Mprime, Qprime, k);
                if (Q[k + 4] ^ Qprime[k + 4]) != DIFFERENCES[k] {
                    truth = false;
                    break;
                }
            }
            if truth {
                let val64 = Q[64] + Q[0];
                let val65 = Q[65] + Q[1];
                let val66 = Q[66] + Q[2];
                let val67 = Q[67] + Q[3];
                let val164 = Qprime[64] + Qprime[0];
                let val165 = Qprime[65] + Qprime[1];
                let val166 = Qprime[66] + Qprime[2];
                let val167 = Qprime[67] + Qprime[3];

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
    let CV: [u32; 4] = block1();
    block2(CV);
}
