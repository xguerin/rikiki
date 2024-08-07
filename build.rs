mod tools {

    use std::{
        env,
        path::{Path, PathBuf},
        process::Command,
    };

    macro_rules! info {
        ($($args:tt)*) => { println!($($args)*) }
    }

    pub fn run(cmd: &mut Command) {
        println!("running: {:?}", cmd);
        let status = match cmd.status() {
            Ok(status) => status,
            Err(e) => panic!("failed to execute command: {}", e),
        };
        if !status.success() {
            panic!(
                "command did not execute successfully: {:?}\n\
                     expected success, got: {}",
                cmd, status
            );
        }
    }

    pub fn build_mnml(num_jobs: &str, out_dir: &PathBuf, bld_dir: &PathBuf, ins_dir: &Path) {
        /*
         * Clone the repository.
         */
        let mut clone = Command::new("git");
        clone
            .current_dir(out_dir)
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg("--branch")
            .arg("v2.3.0")
            .arg("https://github.com/xguerin/minima.l.git");
        run(&mut clone);
        /*
         * Create the build directory.
         */
        std::fs::create_dir(bld_dir).unwrap_or_default();
        /*
         * Grab the profile.
         */
        let profile = env::var("PROFILE").unwrap_or("debug".into()).to_uppercase();
        /*
         * Configure TULIPS.
         */
        let mut cmake = Command::new("cmake");
        cmake
            .current_dir(bld_dir)
            .arg(format!("-DCMAKE_BUILD_TYPE={profile}"))
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", ins_dir.display()))
            .arg("..");
        run(&mut cmake);
        /*
         * Build and install.
         */
        let mut make = Command::new("make");
        make.current_dir(bld_dir)
            .arg("-j")
            .arg(num_jobs)
            .arg("install");
        run(&mut make);
    }

    pub fn build() {
        /*
         * Collect environment variables.
         */
        let num_jobs = env::var("NUM_JOBS").expect("NUM_JOBS was not set");
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR was not set"));
        let src_dir = env::current_dir().expect("failed to get current directory");
        let git_dir = out_dir.join("minima.l");
        let bld_dir = git_dir.join("build");
        let ins_dir = git_dir.join("install");
        /*
         * Print the environment.
         */
        info!("NUM_JOBS={}", num_jobs.clone());
        info!("OUT_DIR={:?}", out_dir);
        info!("SRC_DIR={:?}", src_dir);
        info!("INSTALL_DIR={:?}", ins_dir);
        /*
         * Build.
         */
        if !std::path::Path::new(&ins_dir).exists() {
            std::fs::remove_dir_all(&git_dir).unwrap_or(());
            build_mnml(&num_jobs, &out_dir, &bld_dir, &ins_dir);
        }
        /*
         * Cargo configuration.
         */
        println!("cargo:rustc-link-search=native={}/lib", ins_dir.display());
        println!("cargo:rustc-link-lib=dylib=minimal");
        println!("cargo:rerun-if-changed=build.rs");
    }
}

fn main() {
    tools::build();
}
