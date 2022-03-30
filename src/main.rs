use std::fs::File;
use std::io::Write;
use std::process;
use std::thread;
use std::time::SystemTime;

mod block_1;
mod block_2;

const NTHREADS: u32 = 10;

fn calculate_m_and_mprime(init_vector: [u32; 4]) -> ([u32; 4], [u32; 32], [u32; 32]) {
    println!(
        "Starting new computation - IV: 0x{:x} 0x{:x} 0x{:x} 0x{:x}",
        init_vector[0], init_vector[1], init_vector[2], init_vector[3]
    );
    let start = SystemTime::now();
    let mut cv_and_blocks1: ([u32; 4], [u32; 32], [u32; 32]) = block_1::block1(init_vector);
    let cv_and_blocks2: ([u32; 4], [u32; 16], [u32; 16]) = block_2::block2(cv_and_blocks1.0);

    let end = SystemTime::now();
    let elapsed = end.duration_since(start);
    println!(
        "\n\tExecution time: {} secs\n",
        elapsed.unwrap_or_default().as_secs()
    );

    for i in 16..32 {
        cv_and_blocks1.1[i] = cv_and_blocks2.1[i - 16];
        cv_and_blocks1.2[i] = cv_and_blocks2.2[i - 16];
    }

    return (cv_and_blocks2.0, cv_and_blocks1.1, cv_and_blocks1.2);
}

fn add_to_output(block: [u32; 32], out: &mut Vec<u8>) {
    for hex in block {
        let mut ii: [u8; 4] = hex.to_be_bytes();
        ii.reverse();
        for el in ii {
            out.push(el);
        }
    }
}

fn main() {
    println!("---==[md5ium]==---");
    let default_iv: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];

    let mut children_thread = vec![];

    for _ in 0..NTHREADS {
        children_thread.push(thread::spawn(move || {
            let cv_two_blocks = calculate_m_and_mprime(default_iv);

            // Converting the blocks to the proper format
            let mut output_file_1: Vec<u8> = Vec::new();
            let mut output_file_2: Vec<u8> = Vec::new();

            add_to_output(cv_two_blocks.1, &mut output_file_1);
            add_to_output(cv_two_blocks.2, &mut output_file_2);

            assert_ne!(output_file_1, output_file_2);
            // Writing the blocks to a file
            let mut f1 = File::create("b1.bin").unwrap();
            f1.write_all(output_file_1.as_slice()).unwrap();
            let mut f2 = File::create("b2.bin").unwrap();
            f2.write_all(output_file_2.as_slice()).unwrap();

            process::exit(0); // exit before waiting for the other threads to finish
        }));
    }

    for thread in children_thread {
        // this should never happen because of the exit
        let _ = thread.join();
    }
}
