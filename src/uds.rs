use std::os::unix::net::UnixStream as StdUnixStream;

pub fn connect_ctrl(ctrl_path: &str) -> libc::c_int {
    loop {
        match StdUnixStream::connect(ctrl_path) {
            Ok(s) => {
                use std::os::unix::io::IntoRawFd;
                return s.into_raw_fd();
            }
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    }
}
