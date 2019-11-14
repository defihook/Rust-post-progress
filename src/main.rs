use std::error::Error;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::thread::sleep;
use std::{fs, time};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "ex-post-progress")]
struct Opt {
    pid: u64,
    path: PathBuf,
}

fn find_fd_for_open_file(pid: u64, path: &PathBuf) -> Result<u32, Box<dyn Error>> {
    // - Find file descriptor
    // 	- ls fd
    // 	- readlink on each, if it matches path, you got it
    // 	- error if not
    for dir_entry in fs::read_dir(format!("/proc/{}/fd/", pid))? {
        let dir_entry = dir_entry?;
        if &dir_entry.path().read_link()? == path {
            return Ok(dir_entry
                .file_name()
                .into_string()
                .unwrap()
                .parse::<u32>()
                .unwrap());
        }
    }
    panic!(format!("Couldn't find fd pointing to: {:?}", path))
}

fn get_pos_from_fdinfo(contents: &str) -> u64 {
    for line in contents.lines() {
        if line.starts_with("pos:") {
            let mut pieces = line.split('\t');
            pieces.next();
            return pieces.next().unwrap().parse::<u64>().unwrap();
        }
    }
    panic!("Couldn't parse fdinfo")
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let absolute_path = fs::canonicalize(&opt.path)?;
    let fd = find_fd_for_open_file(opt.pid, &absolute_path)?;

    let file_size = fs::metadata(format!("/proc/{}/fd/{}", opt.pid, fd))?.len();

    let pb = indicatif::ProgressBar::new(file_size);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .progress_chars("#>-"),
    );

    let mut fdinfo = fs::File::open(format!("/proc/{}/fdinfo/{}", opt.pid, fd))?;
    loop {
        let mut contents = "".to_string();
        fdinfo.read_to_string(&mut contents)?;
        fdinfo.seek(SeekFrom::Start(0))?;

        let pos = get_pos_from_fdinfo(&contents);
        pb.set_position(pos);
        if pos == file_size {
            break;
        }

        sleep(time::Duration::from_millis(100));
    }

    Ok(())
}
