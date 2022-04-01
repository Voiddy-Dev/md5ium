use rand::Rng;

use std::fs::File;
use std::io::{BufRead, BufReader};

#[path = "md5_values.rs"]
mod md5_values;

const DIFFERENCES: [u32; 64] = [
    0x82000000, 0x82000020, 0xfe3f18e0, 0x8600003e, 0x80001fc1, 0x80330000, 0x980003c0, 0x87838000,
    0x800003c3, 0x80001000, 0x80000000, 0x800fe080, 0xff000000, 0x80000000, 0x80008008, 0xa0000000,
    0x80000000, 0x80000000, 0x80020000, 0x80000000, 0x80000000, 0x80000000, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000,
    0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000,
    0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x80000000,
    0x80000000, 0x80000000, 0x80000000, 0x80000000, 0x82000000, 0x82000000, 0x86000000,
];

// Helper functions
#[inline]
fn get_md5_round_func(b: u32, c: u32, d: u32, i: usize) -> u32 {
    if i < 16 {
        return md5_values::md5_f(b, c, d);
    }
    if i < 32 {
        return md5_values::md5_g(b, c, d);
    }
    if i < 48 {
        return md5_values::md5_h(b, c, d);
    }
    return md5_values::md5_i(b, c, d);
}

#[inline]
fn shifting(bit: u32) -> u32 {
    return 1 << (bit - 1);
}

#[inline]
fn phi(q_cond_nodes: &mut [u32; 68], i: usize) -> u32 {
    return get_md5_round_func(
        q_cond_nodes[i - 1],
        q_cond_nodes[i - 2],
        q_cond_nodes[i - 3],
        i - 4,
    );
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

fn modif_for_cond(q_cond_nodes: &mut [u32; 68], q_index: i32, cond: [[u32; 3]; 309]) {
    let start;
    let end;

    match q_index {
        0 => {
            //Q[7-10]
            start = 145;
            end = 211;
        }
        2 => {
            //Q[0,1]
            start = 0;
            end = 52;
        }
        _ => {
            //Q[0-15]
            start = 0;
            end = 274;
        }
    }
    for mut i in start..end {
        let j = cond[i][0] + 4;
        let mut zero_bit: u32 = 0xffffffff;
        let mut one_bit: u32 = 0;
        while cond[i][0] == j - 4 {
            let bit = cond[i][1];
            match cond[i][2] {
                0 => {
                    zero_bit &= !shifting(bit);
                }
                1 => {
                    one_bit |= shifting(bit);
                }
                2 => {
                    if (q_cond_nodes[j as usize - 1] & shifting(bit)) != 0 {
                        one_bit |= shifting(bit);
                    } else {
                        zero_bit &= !shifting(bit);
                    }
                }
                3 => {
                    if (q_cond_nodes[j as usize - 2] & shifting(bit)) != 0 {
                        one_bit |= shifting(bit);
                    } else {
                        zero_bit &= !shifting(bit);
                    }
                }
                4 => {
                    if (q_cond_nodes[j as usize - 1] & shifting(bit)) == 0 {
                        one_bit |= shifting(bit);
                    } else {
                        zero_bit &= !shifting(bit);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        i -= 1;
        q_cond_nodes[j as usize] |= one_bit;
        q_cond_nodes[j as usize] &= zero_bit;
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

fn recalc_0_15(q_cond_nodes: &mut [u32; 68], m_block: &mut [u32; 16], m_prime_block: &mut [u32; 16]) {
    for i in 4..20 {
        m_block[i - 4] = md5_rr(
            q_cond_nodes[i].wrapping_sub(q_cond_nodes[i - 1]),
            md5_values::SMAP[i - 4],
        )
        .wrapping_sub(md5_values::TMAP[i - 4])
        .wrapping_sub(q_cond_nodes[i - 4])
        .wrapping_sub(phi(q_cond_nodes, i));
        m_prime_block[i - 4] = m_block[i - 4];
    }
    m_prime_block[4] = m_prime_block[4].wrapping_sub(0x80000000);
    m_prime_block[11] = m_prime_block[11].wrapping_sub(0x8000);
    m_prime_block[14] = m_prime_block[14].wrapping_sub(0x80000000);
}

fn md5_20_steps(
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
        t = a.wrapping_add(
            ((b & c) | ((!b) & d))
                .wrapping_add(m_block[md5_values::MMAP[j] as usize])
                .wrapping_add(md5_values::TMAP[j]),
        );
        let temp = d;
        d = c;
        c = b;
        a = temp;
        t1 = t >> (32 - md5_values::SMAP[j]);
        b = b.wrapping_add((t << md5_values::SMAP[j]).wrapping_add(t1));
        vals[j + 4] = b;
    }
    for j in 16..21 {
        t = a
            .wrapping_add((b & d) | (c & !d))
            .wrapping_add(m_block[md5_values::MMAP[j] as usize])
            .wrapping_add(md5_values::TMAP[j]);
        let temp = d;
        d = c;
        c = b;
        a = temp;
        t1 = t >> (32 - md5_values::SMAP[j]);
        b = b.wrapping_add((t << md5_values::SMAP[j]).wrapping_add(t1));
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
            .wrapping_add((b & c) | ((!b) & d))
            .wrapping_add(m_prime_block[md5_values::MMAP[j] as usize])
            .wrapping_add(md5_values::TMAP[j]);
        let temp = d;
        d = c;
        c = b;
        a = temp;
        t1 = t >> (32 - md5_values::SMAP[j]);
        b = b.wrapping_add((t << md5_values::SMAP[j]) + t1);
        vals1[j + 4] = b;
    }
    for j in 16..21 {
        t = a
            .wrapping_add((b & d) | (c & !d))
            .wrapping_add(m_prime_block[md5_values::MMAP[j] as usize])
            .wrapping_add(md5_values::TMAP[j]);
        let temp = d;
        d = c;
        c = b;
        a = temp;
        t1 = t >> (32 - md5_values::SMAP[j]);
        b = b.wrapping_add((t << md5_values::SMAP[j]).wrapping_add(t1));
        vals1[j + 4] = b;
    }
}

fn verify_conditions(q_cond_nodes: [u32; 68], start: i32, end: i32, cond: [[u32; 3]; 309]) -> bool {
    // Goes through every condition and verifies if they are all satisfied
    for mut i in start..end {
        let j = cond[i as usize][0] + 4;
        let mut zero_bit: u32 = 0xffffffff;
        let mut one_bit: u32 = 0;
        while cond[i as usize][0] == j - 4 {
            // " y is the bit within the step value
            //   that the condition is placed on    "
            let bit = cond[i as usize][1];
            match cond[i as usize][2] {
                // match on condition type
                0 => {
                    zero_bit &= !shifting(bit);
                }
                1 => {
                    one_bit |= shifting(bit);
                }
                2 => {
                    if (q_cond_nodes[j as usize - 1] & shifting(bit)) != 0 {
                        one_bit |= shifting(bit);
                    } else {
                        zero_bit &= !shifting(bit);
                    }
                }
                3 => {
                    if (q_cond_nodes[j as usize - 2] & shifting(bit)) != 0 {
                        one_bit |= shifting(bit);
                    } else {
                        zero_bit &= !shifting(bit);
                    }
                }
                4 => {
                    if (q_cond_nodes[j as usize - 1] & shifting(bit)) == 0 {
                        one_bit |= shifting(bit);
                    } else {
                        zero_bit &= !shifting(bit);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        i -= 1;

        if (q_cond_nodes[j as usize] != (q_cond_nodes[j as usize] | one_bit))
            || (q_cond_nodes[j as usize] != (q_cond_nodes[j as usize] & zero_bit))
        {
            return false;
        }
    }
    return true;
}

pub fn block2(chaining_value: [u32; 4]) -> ([u32; 4], [u32; 16], [u32; 16]) {
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
                modif_for_cond(&mut q_cond_nodes, 1, cond);
                recalc_0_15(&mut q_cond_nodes, &mut m_block, &mut m_prime_block);
                if ((m_block[4] | m_block[14]) & 0x80000000) != 0 && (m_block[11] & 0x8000) != 0 {
                    md5_20_steps(
                        &mut m_block,
                        &mut q_cond_nodes,
                        &mut m_prime_block,
                        &mut q_prime,
                    );
                    if (q_cond_nodes[19] ^ q_prime[19]) == 0xa0000000 {
                        if verify_conditions(q_cond_nodes, 0, 274, cond) {
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
                modif_for_cond(&mut q_cond_nodes, 2, cond);
                recalc_0_15(&mut q_cond_nodes, &mut m_block, &mut m_prime_block);
                md5_20_steps(
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
                    && verify_conditions(q_cond_nodes, 0, 286, cond)
                {
                    c = false;
                    b = false;
                }
            }
        }

        msg_found = multi_msg(
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

            return (
                [
                    q_cond_nodes[64] + q_cond_nodes[0],
                    q_cond_nodes[67] + q_cond_nodes[3],
                    q_cond_nodes[66] + q_cond_nodes[2],
                    q_cond_nodes[65] + q_cond_nodes[1],
                ],
                m_block,
                m_prime_block,
            );
        }
    }
    panic!("Block 2 failed");
}

fn md5_1_step(
    m_block: &mut [u32; 16],
    out: &mut [u32; 68],
    m_prime_block: &mut [u32; 16],
    out1: &mut [u32; 68],
    j: usize,
) {
    let mut t;
    let mut t1;
    t = out[j]
        .wrapping_add(get_md5_round_func(out[j + 3], out[j + 2], out[j + 1], j))
        .wrapping_add(m_block[md5_values::MMAP[j] as usize])
        .wrapping_add(md5_values::TMAP[j]);
    t1 = t >> (32 - md5_values::SMAP[j]);
    t1 = out[j + 3].wrapping_add((t << md5_values::SMAP[j]).wrapping_add(t1));
    out[j + 4] = t1;
    t = out1[j]
        .wrapping_add(get_md5_round_func(out1[j + 3], out1[j + 2], out1[j + 1], j))
        .wrapping_add(m_prime_block[md5_values::MMAP[j] as usize])
        .wrapping_add(md5_values::TMAP[j]);
    t1 = t >> (32 - md5_values::SMAP[j]);
    t1 = out1[j + 3].wrapping_add((t << md5_values::SMAP[j]).wrapping_add(t1));
    out1[j + 4] = t1;
}

#[allow(unused_assignments)]
fn multi_msg(
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

            q_cond_nodes[15] = q_cond_nodes[14].wrapping_add(md5_rl(
                phi(q_cond_nodes, 15)
                    .wrapping_add(0x895cd7be)
                    .wrapping_add(m_block[11])
                    .wrapping_add(q_cond_nodes[11]),
                22,
            ));

            if (q_cond_nodes[15] & 0xfff81fff) == q_cond_nodes[15]
                && (q_cond_nodes[15] | 0x00081080) == q_cond_nodes[15]
                && ((q_cond_nodes[14] ^ q_cond_nodes[15]) & 0xff000000) == 0
            {
                for i in 7..16 {
                    m_block[i] = md5_rr(
                        q_cond_nodes[i + 4].wrapping_sub(q_cond_nodes[i + 3]),
                        md5_values::SMAP[i],
                    )
                    .wrapping_sub(md5_values::TMAP[i])
                    .wrapping_sub(q_cond_nodes[i])
                    .wrapping_sub(phi(q_cond_nodes, i + 4));
                }
                for v in 7..16 {
                    m_prime_block[v] = m_block[v];
                }
                m_prime_block[11] = m_prime_block[11].wrapping_sub(0x8000);
                m_prime_block[14] = m_prime_block[14].wrapping_sub(0x80000000);
                md5_20_steps(m_block, q_cond_nodes, m_prime_block, q_prime);
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
                    q_cond_nodes[p + 4].wrapping_sub(q_cond_nodes[p + 3]),
                    md5_values::SMAP[p],
                )
                .wrapping_sub(md5_values::TMAP[p])
                .wrapping_sub(q_cond_nodes[p])
                .wrapping_sub(phi(q_cond_nodes, p + 4));
                m_prime_block[p] = m_block[p];
            }
            m_prime_block[11] = m_prime_block[11] - 0x8000;
            md5_20_steps(m_block, q_cond_nodes, m_prime_block, q_prime);
            for k in 21..64 {
                md5_1_step(m_block, q_cond_nodes, m_prime_block, q_prime, k);
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
