use rand::Rng;
use std::fs::File;
use std::io::{BufRead, BufReader};

const IV: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
const RELATIVE_INDEX: usize = 4;

// Step-dependent constant values
const Smap: [i32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];
const Mmap: [u8; 64] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 6, 11, 0, 5, 10, 15, 4, 9, 14, 3, 8,
    13, 2, 7, 12, 5, 8, 11, 14, 1, 4, 7, 10, 13, 0, 3, 6, 9, 12, 15, 2, 0, 7, 14, 5, 12, 3, 10, 1,
    8, 15, 6, 13, 4, 11, 2, 9,
];
const Tmap: [u32; 64] = [
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
fn fix_n19(G_N19: &mut u32) {
    if get_bit(*G_N19, 12) == 1 {
        *G_N19 = addsub_bit(*G_N19, 12, 1);
    }
    if get_bit(*G_N19, 26) == 1 {
        *G_N19 = addsub_bit(*G_N19, 26, 1);
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

#[derive(Debug)]
struct Condition {
    ind: i32,
    cref: i32,
    crind: i32,
    add_const: i32,
}

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
        Smap[index as usize],
    )
    .overflowing_sub(N[RELATIVE_INDEX + i4 as usize].val)
    .0
    .overflowing_sub(F(
        N[RELATIVE_INDEX + i1 as usize].val,
        N[RELATIVE_INDEX + i2 as usize].val,
        N[RELATIVE_INDEX + i3 as usize].val,
    ))
    .0
    .overflowing_sub(Tmap[index as usize])
    .0;
}

fn build_bitfield(N: &mut Vec<Node>) {
    let mut count = 0;
    for el in N {
        if count >= RELATIVE_INDEX {
            let mut list_iter = el.list.iter();
            while let Some(li) = list_iter.next() {
                // if count == 20 {
                //     println!("{:?}", li);
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
            }
            // for li in &el.list {
            //     if li.cref == -1 {
            //         el.bf[0] = addsub_bit(el.bf[0], li.ind, 1);
            //     }
            //     if li.cref == -2 {
            //         el.bf[1] = addsub_bit(el.bf[1], li.ind, 1);
            //     }
            //     if (li.cref == -1) && (li.add_const == 0) {
            //         el.bf[2] = addsub_bit(el.bf[2], li.crind, 1);
            //     }
            //     if (li.cref == -1) && (li.add_const != 0) {
            //         el.bf[3] = addsub_bit(el.bf[3], li.crind, 1);
            //     }
            // }
        }
        count += 1;
    }
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
                    .overflowing_add(Tmap[i])
                    .0,
                    Smap[i],
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
                    .overflowing_add(Tmap[i])
                    .0,
                    Smap[i],
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
                    .overflowing_add(Tmap[i])
                    .0,
                    Smap[i],
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
            }
        }
        M[4] = addsub_bit(M[4], 31, -1);
        M[11] = addsub_bit(M[11], 15, -1);
        M[14] = addsub_bit(M[14], 31, -1);
        // println!("{} {} {}", M[4], M[11], M[14]);
        // panic!();
    }
    println!("Done with first round");
}

fn new_randM(M: &mut [u32; 32]) {
    let mut rng = rand::thread_rng();
    for i in 0..16 {
        M[i] = rng.gen();
    }
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
                ^ (N[RELATIVE_INDEX + ind as usize].val & N[RELATIVE_INDEX + ind as usize].bf[2]);
            }
            _ => {
                panic!("BRUV LI MUST BE SOME");
            }
        }
       
    }
    // println!("\tMOTHERFUCKING FCHECK --> {}", x);
    x
}

// fn check_cond(ind: i32, N: &mut Vec<Node>) -> i32 {
//     let mut b1: u32;
//     let mut b2: u32;

//     for (_, list) in N[RELATIVE_INDEX + ind as usize].list.iter().enumerate() {
//         // println!("here {}", list.ind);
//         // println!("CONDITION {:?}", list);
//         // get bit value at list->ind
//         b1 = get_bit(N[RELATIVE_INDEX + ind as usize].val, list.ind);
//         // println!("b1 {}", b1);
//         if list.cref < 0 {
//             if b1 != (list.cref + 2) as u32 {
//                 return 0;
//             }
//         } else {
//             b2 = get_bit(N[RELATIVE_INDEX + list.cref as usize].val, list.crind);
//             if b1 != (b2 ^ list.add_const as u32) {
//                 return 0;
//             }
//         }
//     }

//     1
// }

fn klima1_3(M: &mut [u32; 32], N: &mut Vec<Node>) -> bool {
    let mut rng = rand::thread_rng();
    let mut x: u32;

    println!("AT KLIMA 1_3");

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
                .overflowing_add(Tmap[17])
                .0,
                Smap[17],
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
                .overflowing_add(Tmap[18])
                .0,
                Smap[18],
            ))
            .0;
    }
    false
}

fn klima4_9(M: &mut [u32; 32], N: &mut Vec<Node>, G_N19: &mut u32) {
    N[RELATIVE_INDEX + 19].val = *G_N19;
    *G_N19 += 1;
    fix_n19(G_N19);
    // println!("G_N19 {}", G_N19);
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
                .overflowing_add(Tmap[0])
                .0,
            Smap[0],
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
                .overflowing_add(Tmap[1])
                .0,
            Smap[1],
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
    .overflowing_sub(Tmap[2])
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
    .overflowing_sub(Tmap[3])
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
    .overflowing_sub(Tmap[4])
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
    .overflowing_sub(Tmap[5])
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
        *G_N19 += 0x7f;
        fix_n19(G_N19);
        N[RELATIVE_INDEX + 19].val = *G_N19;
        // println!("{} {}", N[RELATIVE_INDEX + 19].val, *G_N19);
    }
    // println!(" ========= end klima 4_9 ========= ");
}

fn first_block(M: &mut [u32; 32], N: &mut Vec<Node>, dt: [u32; 68], G_N19: &mut u32) {
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
    first_round(M, N, dt);

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
    klima4_9(M, N, G_N19);
    let mut stepno = check_diffs(M, N, 20, dt);
    println!("Stepno {}", stepno);
    // panic!();

    while stepno >= 0 {
        if *G_N19 >= 0x80000000 {
            println!("\tG TOO MUCH {}", stepno);
            *G_N19 = 0;
            while klima1_3(M, N) {
                new_randM(M);
                first_round(M, N, dt);
            }
        }
        // iterate over values of N[19]
        klima4_9(M, N, G_N19);
        stepno = check_diffs(M, N, 20, dt);
        // println!("Stepno {} - G_N19 {}", stepno, G_N19);
    }
    println!("BRUV SUCC - stepno {}", stepno);
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
                    .overflowing_add(M[Mmap[i] as usize])
                    .0
                    .overflowing_add(Tmap[i])
                    .0,
                    Smap[i],
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
            //         .overflowing_add(Mprime[Mmap[i] as usize])
            //         .0
            //         .overflowing_add(Tmap[i])
            //         .0,
            //         Smap[i],
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
                    .overflowing_add(Mprime[Mmap[i] as usize])
                    .0
                    .overflowing_add(Tmap[i])
                    .0,
                    Smap[i],
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
                .overflowing_add(M[Mmap[i] as usize])
                .0
                .overflowing_add(Tmap[i])
                .0,
                Smap[i],
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
                .overflowing_add(Mprime[Mmap[i] as usize])
                .0
                .overflowing_add(Tmap[i])
                .0,
                Smap[i],
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
                .overflowing_add(M[Mmap[i] as usize])
                .0
                .overflowing_add(Tmap[i])
                .0,
                Smap[i],
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
                .overflowing_add(Mprime[Mmap[i] as usize])
                .0
                .overflowing_add(Tmap[i])
                .0,
                Smap[i],
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
                .overflowing_add(M[Mmap[i] as usize])
                .0
                .overflowing_add(Tmap[i])
                .0,
                Smap[i],
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
                .overflowing_add(Mprime[Mmap[i] as usize])
                .0
                .overflowing_add(Tmap[i])
                .0,
                Smap[i],
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
                .overflowing_add(M[Mmap[i] as usize])
                .0
                .overflowing_add(Tmap[i])
                .0,
                Smap[i],
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
                .overflowing_add(Mprime[Mmap[i] as usize])
                .0
                .overflowing_add(Tmap[i])
                .0,
                Smap[i],
            ))
            .0;
    }

    // Calculate new chaining variables
    N[RELATIVE_INDEX + 68].val = N[RELATIVE_INDEX + 60].val.overflowing_add(N[RELATIVE_INDEX - 4].val).0;
    N[RELATIVE_INDEX + 69].val = N[RELATIVE_INDEX + 61].val.overflowing_add(N[RELATIVE_INDEX - 3].val).0;
    N[RELATIVE_INDEX + 70].val = N[RELATIVE_INDEX + 62].val.overflowing_add(N[RELATIVE_INDEX - 2].val).0;
    N[RELATIVE_INDEX + 71].val = N[RELATIVE_INDEX + 63].val.overflowing_add(N[RELATIVE_INDEX - 1].val).0;
    N[RELATIVE_INDEX + 68].Tval = N[RELATIVE_INDEX + 60].Tval.overflowing_add(N[RELATIVE_INDEX - 4].val).0;
    N[RELATIVE_INDEX + 69].Tval = N[RELATIVE_INDEX + 61].Tval.overflowing_add(N[RELATIVE_INDEX - 3].val).0;
    N[RELATIVE_INDEX + 70].Tval = N[RELATIVE_INDEX + 62].Tval.overflowing_add(N[RELATIVE_INDEX - 2].val).0;
    N[RELATIVE_INDEX + 71].Tval = N[RELATIVE_INDEX + 63].Tval.overflowing_add(N[RELATIVE_INDEX - 1].val).0;

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

fn block1() {
    let mut G_N19: u32 = 0;

    let mut rng = rand::thread_rng();

    // Building condition list and bitfield
    let mut re = build_condition_list("./data/md5cond_1.txt".to_string());
    build_bitfield(&mut re);
    let dt = construct_diff_table();
    let mut M: [u32; 32] = [0; 32];
    for i in 0..16 {
        M[i] = rng.gen();
    }
    println!("\n\tBefore CV: {:x}{:x}{:x}{:x}", re[68].val, re[71].val, re[70].val, re[69].val);
    first_block(&mut M, &mut re, dt, &mut G_N19);
    while check_diffs(&mut M, &mut re, 0, dt) > -1 {
        first_block(&mut M, &mut re, dt, &mut G_N19);
    }
    println!("\n\tChaining value: {:x}{:x}{:x}{:x}", re[68].val, re[71].val, re[70].val, re[69].val);

    // Printing out message
    print!("M\t");
    for i in 0..15 {
        if i % 4 == 0 && i != 0 {
            print!("\n\t");
        }
        print!("{:x}, ", M[i]);
    }
    print!("{:x}\n\n", M[15]);
    M[ 4] = addsub_bit(M[ 4], 31, 1);
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
}

fn main() {
    println!("---==[md5ium]==---");
    block1();
}
