use std::error::Error;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::{fs, thread, time};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "ex-post-progress")]
struct Opt {
    pid: u64,
    path: PathBuf,
}

fn find_fds_for_open_file(pid: u64, path: &PathBuf) -> Result<Vec<u32>, Box<dyn Error>> {
    let mut fds = vec![];
    for dir_entry in fs::read_dir(format!("/proc/{}/fd/", pid))? {
        let dir_entry = dir_entry?;
        if &dir_entry.path().read_link()? == path {
            fds.push(
                dir_entry
                    .file_name()
                    .into_string()
                    .unwrap()
                    .parse::<u32>()
                    .unwrap(),
            );
        }
    }
    Ok(fds)
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
    let pid = opt.pid;
    let fds = find_fds_for_open_file(pid, &absolute_path)?;

    let m = indicatif::MultiProgress::new();

    for fd in fds {
        let file_size = fs::metadata(format!("/proc/{}/fd/{}", pid, fd))?.len();
        let pb = m.add(indicatif::ProgressBar::new(file_size));
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
                .progress_chars("#>-"),
        );
        pb.set_message(&format!("/proc/{}/fd/{}", pid, fd));
        let mut fdinfo = fs::File::open(format!("/proc/{}/fdinfo/{}", pid, fd))?;
        thread::spawn(move || {
            loop {
                let mut contents = "".to_string();
                fdinfo.read_to_string(&mut contents).unwrap();
                fdinfo.seek(SeekFrom::Start(0)).unwrap();

                let pos = get_pos_from_fdinfo(&contents);
                pb.set_position(pos);
                if pos == file_size {
                    break;
                }
                thread::sleep(time::Duration::from_millis(100));
            }
            pb.finish_with_message("done");
        });
    }
    m.join_and_clear().unwrap();

    Ok(())
}
