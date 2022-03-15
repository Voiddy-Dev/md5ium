use std::fs::File;
use std::io::Write;
use std::time::SystemTime;

mod block_1;
mod block_2;

fn main() {
    println!("---==[md5ium]==---");
    let start = SystemTime::now();
    let mut cv_and_blocks1: ([u32; 4], [u32; 32], [u32; 32]) = block_1::block1();
    let blocks2: ([u32; 16], [u32; 16]) = block_2::block2(cv_and_blocks1.0);

    let end = SystemTime::now();
    let elapsed = end.duration_since(start);
    println!(
        "Execution time: {} secs",
        elapsed.unwrap_or_default().as_secs()
    );

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
