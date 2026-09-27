fn main() {
    #[cfg(windows)]
    {
        let res = winresource::WindowsResource::new();
        // res.set_icon("assets/app-icon.ico"); // در صورت داشتن آیکون exe
        res.compile().unwrap();
    }
}
