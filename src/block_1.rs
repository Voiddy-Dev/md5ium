use rand::Rng;

use std::fs::File;
use std::io::{BufRead, BufReader};

#[path = "md5_values.rs"]
mod md5_values;

const RELATIVE_INDEX: usize = 4;

#[inline]
fn cls(x: u32, s: i32) -> u32 {
    (x << s) ^ (x >> (32 - s))
}
#[inline]
fn crs(x: u32, s: i32) -> u32 {
    (x >> s) ^ (x << (32 - s))
}

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
        t = b.wrapping_mul(res);
    }
    // x + t
    let return_val = x.wrapping_add(t as u32);
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
    #[inline]
    fn default() -> Node {
        Node {
            val: 0,
            tval: 0,
            bf: [0, 0, 0, 0],
            list: Vec::new(),
        }
    }
}

fn single_msg_modif(index: i32, n_cond_nodes: &mut [Node; 76]) -> u32 {
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
    return crs(
        x.wrapping_sub(n_cond_nodes[RELATIVE_INDEX + i1 as usize].val),
        md5_values::SMAP[index as usize],
    )
    .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + i4 as usize].val)
    .wrapping_sub(md5_values::md5_f(
        n_cond_nodes[RELATIVE_INDEX + i1 as usize].val,
        n_cond_nodes[RELATIVE_INDEX + i2 as usize].val,
        n_cond_nodes[RELATIVE_INDEX + i3 as usize].val,
    ))
    .wrapping_sub(md5_values::TMAP[index as usize]);
}

fn build_bitfield(n_cond_nodes: &mut [Node; 76]) {
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
    diff_table[64] += addsub_bit(0, 31, 1);
    diff_table[65] += addsub_bit(0, 31, 1);
    diff_table[65] += addsub_bit(0, 25, 1);
    diff_table[66] += addsub_bit(0, 31, 1);
    diff_table[66] += addsub_bit(0, 25, 1);
    diff_table[67] += addsub_bit(0, 31, -1);
    diff_table[67] += addsub_bit(0, 25, 1);

    diff_table
}

/// Function to satisfy first round differentials
fn first_round(m_block: &mut [u32; 32], n_cond_nodes: &mut [Node; 76], diff_table: [u32; 68]) {
    loop {
        let mut breakout = true;
        // Go through the first round values
        for i in 0..16 {
            n_cond_nodes[RELATIVE_INDEX + i].val =
                n_cond_nodes[RELATIVE_INDEX + i - 1].val.wrapping_add(cls(
                    md5_values::md5_f(
                        n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                    )
                    .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                    .wrapping_add(m_block[i])
                    .wrapping_add(md5_values::TMAP[i]),
                    md5_values::SMAP[i],
                ));
            m_block[i] = single_msg_modif(i as i32, n_cond_nodes);
            n_cond_nodes[RELATIVE_INDEX + i].val =
                n_cond_nodes[RELATIVE_INDEX + i - 1].val.wrapping_add(cls(
                    md5_values::md5_f(
                        n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                        n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                    )
                    .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                    .wrapping_add(m_block[i])
                    .wrapping_add(md5_values::TMAP[i]),
                    md5_values::SMAP[i],
                ));

            if i == 4 || i == 14 {
                m_block[i] = addsub_bit(m_block[i], 31, 1);
            } else if i == 11 {
                m_block[i] = addsub_bit(m_block[i], 15, 1);
            }

            n_cond_nodes[RELATIVE_INDEX + i].tval =
                n_cond_nodes[RELATIVE_INDEX + i - 1].tval.wrapping_add(cls(
                    md5_values::md5_f(
                        n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                        n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                        n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                    )
                    .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                    .wrapping_add(m_block[i])
                    .wrapping_add(md5_values::TMAP[i]),
                    md5_values::SMAP[i],
                ));

            if n_cond_nodes[RELATIVE_INDEX + i]
                .tval
                .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + i].val)
                != diff_table[i]
            {
                // differential is not satisfied
                new_rand_mblock(m_block);
                if i == 4 || i == 14 {
                    m_block[i] = addsub_bit(m_block[i], 31, -1);
                } else if i == 11 {
                    m_block[i] = addsub_bit(m_block[i], 15, -1);
                }
                breakout = false;
                break;
            }
            if i == 4 || i == 14 {
                m_block[i] = addsub_bit(m_block[i], 31, -1);
            } else if i == 11 {
                m_block[i] = addsub_bit(m_block[i], 15, -1);
            }
        }
        if breakout {
            break;
        }
        // m_block[4] = addsub_bit(m_block[4], 31, 1);
        // m_block[11] = addsub_bit(m_block[11], 15, 1);
        // m_block[14] = addsub_bit(m_block[14], 31, 1);

        // for i in 0..16 {
        //     n_cond_nodes[RELATIVE_INDEX + i].tval =
        //         n_cond_nodes[RELATIVE_INDEX + i - 1].tval.wrapping_add(cls(
        //             md5_values::md5_f(
        //                 n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
        //                 n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
        //                 n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
        //             )
        //             .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
        //             .wrapping_add(m_block[i])
        //             .wrapping_add(md5_values::TMAP[i]),
        //             md5_values::SMAP[i],
        //         ));

        //     if n_cond_nodes[RELATIVE_INDEX + i]
        //         .tval
        //         .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + i].val)
        //         != diff_table[i]
        //     {
        //         flag = 0;
        //         new_rand_mblock(m_block);
        //     }
        // }
        // m_block[4] = addsub_bit(m_block[4], 31, -1);
        // m_block[11] = addsub_bit(m_block[11], 15, -1);
        // m_block[14] = addsub_bit(m_block[14], 31, -1);
    }
}

#[inline]
fn new_rand_mblock(m_block: &mut [u32; 32]) {
    let mut rng = rand::thread_rng();
    for i in 0..16 {
        m_block[i] = rng.gen();
    }
}

fn fcheck_cond(ind: i32, n_cond_nodes: &mut [Node; 76]) -> u32 {
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
                panic!("LI MUST BE SOME");
            }
        }
    }
    x
}

fn klima1_3(m_block: &mut [u32; 32], n_cond_nodes: &mut [Node; 76]) -> bool {
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
        n_cond_nodes[RELATIVE_INDEX + 17].val =
            n_cond_nodes[RELATIVE_INDEX + 16].val.wrapping_add(cls(
                md5_values::md5_g(
                    n_cond_nodes[RELATIVE_INDEX + 16].val,
                    n_cond_nodes[RELATIVE_INDEX + 15].val,
                    n_cond_nodes[RELATIVE_INDEX + 14].val,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + 13].val)
                .wrapping_add(m_block[6])
                .wrapping_add(md5_values::TMAP[17]),
                md5_values::SMAP[17],
            ));
        n_cond_nodes[RELATIVE_INDEX + 18].val =
            n_cond_nodes[RELATIVE_INDEX + 17].val.wrapping_add(cls(
                md5_values::md5_g(
                    n_cond_nodes[RELATIVE_INDEX + 17].val,
                    n_cond_nodes[RELATIVE_INDEX + 16].val,
                    n_cond_nodes[RELATIVE_INDEX + 15].val,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + 14].val)
                .wrapping_add(m_block[11])
                .wrapping_add(md5_values::TMAP[18]),
                md5_values::SMAP[18],
            ));
    }
    false
}

fn klima4_9(m_block: &mut [u32; 32], n_cond_nodes: &mut [Node; 76], g_n19: &mut u32) {
    n_cond_nodes[RELATIVE_INDEX + 19].val = *g_n19;
    *g_n19 += 1;
    fix_n19(g_n19);

    m_block[0] = crs(
        n_cond_nodes[RELATIVE_INDEX + 19]
            .val
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 18].val),
        20,
    )
    .wrapping_sub(md5_values::md5_g(
        n_cond_nodes[RELATIVE_INDEX + 18].val,
        n_cond_nodes[RELATIVE_INDEX + 17].val,
        n_cond_nodes[RELATIVE_INDEX + 16].val,
    ))
    .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 15].val)
    .wrapping_sub(0xe9b6c7aa);
    n_cond_nodes[RELATIVE_INDEX + 0].val = n_cond_nodes[RELATIVE_INDEX + 67].val.wrapping_add(cls(
        m_block[0]
            .wrapping_add(n_cond_nodes[RELATIVE_INDEX + 64].val)
            .wrapping_add(md5_values::md5_f(
                n_cond_nodes[RELATIVE_INDEX + 67].val,
                n_cond_nodes[RELATIVE_INDEX + 66].val,
                n_cond_nodes[RELATIVE_INDEX + 65].val,
            ))
            .wrapping_add(md5_values::TMAP[0]),
        md5_values::SMAP[0],
    ));
    m_block[1] = crs(
        n_cond_nodes[RELATIVE_INDEX + 16]
            .val
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 15].val),
        5,
    )
    .wrapping_sub(md5_values::md5_g(
        n_cond_nodes[RELATIVE_INDEX + 15].val,
        n_cond_nodes[RELATIVE_INDEX + 14].val,
        n_cond_nodes[RELATIVE_INDEX + 13].val,
    ))
    .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 12].val)
    .wrapping_sub(0xf61e2562);
    n_cond_nodes[RELATIVE_INDEX + 1].val = n_cond_nodes[RELATIVE_INDEX + 0].val.wrapping_add(cls(
        m_block[1]
            .wrapping_add(n_cond_nodes[RELATIVE_INDEX + 65].val)
            .wrapping_add(md5_values::md5_f(
                n_cond_nodes[RELATIVE_INDEX + 0].val,
                n_cond_nodes[RELATIVE_INDEX + 67].val,
                n_cond_nodes[RELATIVE_INDEX + 66].val,
            ))
            .wrapping_add(md5_values::TMAP[1]),
        md5_values::SMAP[1],
    ));
    m_block[2] = crs(
        n_cond_nodes[RELATIVE_INDEX + 2]
            .val
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 1].val),
        17,
    )
    .wrapping_sub(md5_values::md5_f(
        n_cond_nodes[RELATIVE_INDEX + 1].val,
        n_cond_nodes[RELATIVE_INDEX + 0].val,
        n_cond_nodes[RELATIVE_INDEX + 67].val,
    ))
    .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 66].val)
    .wrapping_sub(md5_values::TMAP[2]);
    m_block[3] = crs(
        n_cond_nodes[RELATIVE_INDEX + 3]
            .val
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 2].val),
        22,
    )
    .wrapping_sub(md5_values::md5_f(
        n_cond_nodes[RELATIVE_INDEX + 2].val,
        n_cond_nodes[RELATIVE_INDEX + 1].val,
        n_cond_nodes[RELATIVE_INDEX + 0].val,
    ))
    .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 67].val)
    .wrapping_sub(md5_values::TMAP[3]);
    m_block[4] = crs(
        n_cond_nodes[RELATIVE_INDEX + 4]
            .val
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 3].val),
        7,
    )
    .wrapping_sub(md5_values::md5_f(
        n_cond_nodes[RELATIVE_INDEX + 3].val,
        n_cond_nodes[RELATIVE_INDEX + 2].val,
        n_cond_nodes[RELATIVE_INDEX + 1].val,
    ))
    .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 0].val)
    .wrapping_sub(md5_values::TMAP[4]);
    m_block[5] = crs(
        n_cond_nodes[RELATIVE_INDEX + 5]
            .val
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 4].val),
        12,
    )
    .wrapping_sub(md5_values::md5_f(
        n_cond_nodes[RELATIVE_INDEX + 4].val,
        n_cond_nodes[RELATIVE_INDEX + 3].val,
        n_cond_nodes[RELATIVE_INDEX + 2].val,
    ))
    .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + 1].val)
    .wrapping_sub(md5_values::TMAP[5]);
    n_cond_nodes[RELATIVE_INDEX + 20].val =
        n_cond_nodes[RELATIVE_INDEX + 19].val.wrapping_add(cls(
            m_block[5]
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + 16].val)
                .wrapping_add(md5_values::md5_g(
                    n_cond_nodes[RELATIVE_INDEX + 19].val,
                    n_cond_nodes[RELATIVE_INDEX + 18].val,
                    n_cond_nodes[RELATIVE_INDEX + 17].val,
                ))
                .wrapping_add(0xd62f105d),
            5,
        ));
    if fcheck_cond(20, n_cond_nodes) != 0 {
        *g_n19 += 0x7f;
        fix_n19(g_n19);
        n_cond_nodes[RELATIVE_INDEX + 19].val = *g_n19;
    }
}

fn first_block(
    init_vector: [u32; 4],
    m_block: &mut [u32; 32],
    n_cond_nodes: &mut [Node; 76],
    dt: [u32; 68],
    g_n19: &mut u32,
) {
    n_cond_nodes[RELATIVE_INDEX + 64].val = init_vector[0];
    n_cond_nodes[RELATIVE_INDEX - 4].val = init_vector[0];
    n_cond_nodes[RELATIVE_INDEX - 4].tval = init_vector[0];

    n_cond_nodes[RELATIVE_INDEX + 65].val = init_vector[3];
    n_cond_nodes[RELATIVE_INDEX - 3].val = init_vector[3];
    n_cond_nodes[RELATIVE_INDEX - 3].tval = init_vector[3];

    n_cond_nodes[RELATIVE_INDEX + 66].val = init_vector[2];
    n_cond_nodes[RELATIVE_INDEX - 2].val = init_vector[2];
    n_cond_nodes[RELATIVE_INDEX - 2].tval = init_vector[2];

    n_cond_nodes[RELATIVE_INDEX + 67].val = init_vector[1];
    n_cond_nodes[RELATIVE_INDEX - 1].val = init_vector[1];
    n_cond_nodes[RELATIVE_INDEX - 1].tval = init_vector[1];

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

fn round1_check_diffs(
    m_block: &mut [u32; 32],
    m_prime_block: &mut [u32; 16],
    n_cond_nodes: &mut [Node; 76],
    dt: [u32; 68],
) -> i32 {
    for i in 0..16 {
        n_cond_nodes[RELATIVE_INDEX + i].val =
            n_cond_nodes[RELATIVE_INDEX + i - 1].val.wrapping_add(cls(
                md5_values::md5_f(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                .wrapping_add(m_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));

        n_cond_nodes[RELATIVE_INDEX + i].tval =
            n_cond_nodes[RELATIVE_INDEX + i - 1].tval.wrapping_add(cls(
                md5_values::md5_f(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                .wrapping_add(m_prime_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));

        if n_cond_nodes[RELATIVE_INDEX + i]
            .tval
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + i].val)
            != dt[i]
        {
            return i as i32;
        }
    }
    return 0x1337;
}

fn round2_check_diffs(
    m_block: &mut [u32; 32],
    m_prime_block: &mut [u32; 16],
    n_cond_nodes: &mut [Node; 76],
    ind: usize,
    dt: [u32; 68],
) -> i32 {
    for i in ind..32 {
        n_cond_nodes[RELATIVE_INDEX + i].val =
            n_cond_nodes[RELATIVE_INDEX + i - 1].val.wrapping_add(cls(
                md5_values::md5_g(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                .wrapping_add(m_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));

        n_cond_nodes[RELATIVE_INDEX + i].tval =
            n_cond_nodes[RELATIVE_INDEX + i - 1].tval.wrapping_add(cls(
                md5_values::md5_g(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                .wrapping_add(m_prime_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));
        if n_cond_nodes[RELATIVE_INDEX + i]
            .tval
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + i].val)
            != dt[i]
        {
            return i as i32;
        }
    }
    return 0x1337;
}

fn round3_check_diffs(
    m_block: &mut [u32; 32],
    m_prime_block: &mut [u32; 16],
    n_cond_nodes: &mut [Node; 76]
) -> i32 {
    for i in 32..48 {
        n_cond_nodes[RELATIVE_INDEX + i].val =
            n_cond_nodes[RELATIVE_INDEX + i - 1].val.wrapping_add(cls(
                md5_values::md5_h(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                .wrapping_add(m_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));

        n_cond_nodes[RELATIVE_INDEX + i].tval =
            n_cond_nodes[RELATIVE_INDEX + i - 1].tval.wrapping_add(cls(
                md5_values::md5_h(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                .wrapping_add(m_prime_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));

        if i > 33
            && ((n_cond_nodes[RELATIVE_INDEX + i].tval ^ n_cond_nodes[RELATIVE_INDEX + i].val)
                != 0x80000000)
        {
            return i as i32;
        }
    }
    return 0x1337;
}

fn round4_check_diffs(
    m_block: &mut [u32; 32],
    m_prime_block: &mut [u32; 16],
    n_cond_nodes: &mut [Node; 76],
    dt: [u32; 68],
) -> i32 {
    for i in 48..60 {
        n_cond_nodes[RELATIVE_INDEX + i].val =
            n_cond_nodes[RELATIVE_INDEX + i - 1].val.wrapping_add(cls(
                md5_values::md5_i(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                .wrapping_add(m_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));

        n_cond_nodes[RELATIVE_INDEX + i].tval =
            n_cond_nodes[RELATIVE_INDEX + i - 1].tval.wrapping_add(cls(
                md5_values::md5_i(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                .wrapping_add(m_prime_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));

        if n_cond_nodes[RELATIVE_INDEX + i]
            .tval
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + i].val)
            != dt[i]
        {
            return i as i32;
        }
    }
    return 0x1337;
}

fn check_diffs(
    m_block: &mut [u32; 32],
    n_cond_nodes: &mut [Node; 76],
    index: i32,
    dt: [u32; 68]
) -> i32 {
    let mut m_prime_block: [u32; 16] = [0; 16];
    m_prime_block.copy_from_slice(&m_block[..16]);

    m_prime_block[4] = addsub_bit(m_prime_block[4], 31, 1);
    m_prime_block[11] = addsub_bit(m_prime_block[11], 15, 1);
    m_prime_block[14] = addsub_bit(m_prime_block[14], 31, 1);

    let mut local_index: usize = index as usize;
    if local_index == 20 {
        for i in 15..20 {
            n_cond_nodes[RELATIVE_INDEX + i].tval =
                n_cond_nodes[RELATIVE_INDEX + i].val.wrapping_add(dt[i]);
        }
    }

    if local_index != 20 {
        let ret = round1_check_diffs(m_block, &mut m_prime_block, n_cond_nodes, dt);
        if ret != 0x1337 {
            return ret;
        }
        local_index = 16;
    }

    let ret = round2_check_diffs(m_block, &mut m_prime_block, n_cond_nodes, local_index, dt);
    if ret != 0x1337 {
        return ret;
    }

    let ret = round3_check_diffs(m_block, &mut m_prime_block, n_cond_nodes);
    if ret != 0x1337 {
        return ret;
    }

    let ret = round4_check_diffs(m_block, &mut m_prime_block, n_cond_nodes, dt);
    if ret != 0x1337 {
        return ret;
    }

    for i in 60..64 {
        n_cond_nodes[RELATIVE_INDEX + i].val =
            n_cond_nodes[RELATIVE_INDEX + i - 1].val.wrapping_add(cls(
                md5_values::md5_i(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].val,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].val,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].val)
                .wrapping_add(m_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));

        n_cond_nodes[RELATIVE_INDEX + i].tval =
            n_cond_nodes[RELATIVE_INDEX + i - 1].tval.wrapping_add(cls(
                md5_values::md5_i(
                    n_cond_nodes[RELATIVE_INDEX + i - 1].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 2].tval,
                    n_cond_nodes[RELATIVE_INDEX + i - 3].tval,
                )
                .wrapping_add(n_cond_nodes[RELATIVE_INDEX + i - 4].tval)
                .wrapping_add(m_prime_block[md5_values::MMAP[i] as usize])
                .wrapping_add(md5_values::TMAP[i]),
                md5_values::SMAP[i],
            ));
    }

    n_cond_nodes[RELATIVE_INDEX + 68].val = n_cond_nodes[RELATIVE_INDEX + 60]
        .val
        .wrapping_add(n_cond_nodes[RELATIVE_INDEX - 4].val);
    n_cond_nodes[RELATIVE_INDEX + 69].val = n_cond_nodes[RELATIVE_INDEX + 61]
        .val
        .wrapping_add(n_cond_nodes[RELATIVE_INDEX - 3].val);
    n_cond_nodes[RELATIVE_INDEX + 70].val = n_cond_nodes[RELATIVE_INDEX + 62]
        .val
        .wrapping_add(n_cond_nodes[RELATIVE_INDEX - 2].val);
    n_cond_nodes[RELATIVE_INDEX + 71].val = n_cond_nodes[RELATIVE_INDEX + 63]
        .val
        .wrapping_add(n_cond_nodes[RELATIVE_INDEX - 1].val);
    n_cond_nodes[RELATIVE_INDEX + 68].tval = n_cond_nodes[RELATIVE_INDEX + 60]
        .tval
        .wrapping_add(n_cond_nodes[RELATIVE_INDEX - 4].val);
    n_cond_nodes[RELATIVE_INDEX + 69].tval = n_cond_nodes[RELATIVE_INDEX + 61]
        .tval
        .wrapping_add(n_cond_nodes[RELATIVE_INDEX - 3].val);
    n_cond_nodes[RELATIVE_INDEX + 70].tval = n_cond_nodes[RELATIVE_INDEX + 62]
        .tval
        .wrapping_add(n_cond_nodes[RELATIVE_INDEX - 2].val);
    n_cond_nodes[RELATIVE_INDEX + 71].tval = n_cond_nodes[RELATIVE_INDEX + 63]
        .tval
        .wrapping_add(n_cond_nodes[RELATIVE_INDEX - 1].val);

    for i in 69..72 {
        if fcheck_cond(i, n_cond_nodes) != 0 {
            return i;
        }
        if n_cond_nodes[RELATIVE_INDEX + i as usize]
            .tval
            .wrapping_sub(n_cond_nodes[RELATIVE_INDEX + i as usize].val)
            != dt[i as usize - 4]
        {
            return i as i32;
        }
    }

    return -1;
}

pub fn block1(init_vector: [u32; 4]) -> ([u32; 4], [u32; 32], [u32; 32]) {
    let mut g_n19: u32 = 0;

    // Building condition list and bitfield
    let mut re = build_condition_list("./data/md5cond_1.txt".to_string());
    build_bitfield(&mut re);
    let dt = construct_diff_table();
    // Initial random message
    let mut m_block: [u32; 32] = [0; 32];
    new_rand_mblock(&mut m_block); // Randomize

    first_block(init_vector, &mut m_block, &mut re, dt, &mut g_n19);
    while check_diffs(&mut m_block, &mut re, 0, dt) > -1 {
        first_block(init_vector, &mut m_block, &mut re, dt, &mut g_n19);
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

fn build_condition_list(filename: String) -> [Node; 76] {
    let f = File::open(filename).expect("Errors reading cond file");
    let reader = BufReader::new(f);
    // I am sad I had to do that... but Rust and Copy of Vecs does not work
    let mut res: [Node; 76] = [
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
        Node::default(),
    ];

    for line in reader.lines() {
        match line {
            Ok(l) => {
                let mut split = l.split(" ");
                // Getting index
                let mut q_index: usize = split.next().unwrap().parse().unwrap();
                q_index += RELATIVE_INDEX;
                assert!(q_index < 76);

                let cond = Condition {
                    ind: split.next().unwrap().parse().unwrap(),
                    cref: split.next().unwrap().parse().unwrap(),
                    crind: split.next().unwrap().parse().unwrap(),
                    add_const: split.next().unwrap().parse().unwrap(),
                };
                res[q_index].list.push(cond);
                res[q_index].list.sort_by(|b, a| b.ind.cmp(&a.ind));
            }
            _ => print!("Error in line."),
        }
    }
    res
}
