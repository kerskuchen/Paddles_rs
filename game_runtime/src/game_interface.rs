use game_lib::{AudioContext, GameContext, GameInput};
use libloading::Library;
use std;

// TODO(JaSc): Use std::path::Paths instead of Strings for better readability

/// This helper struct provides convenience methods to load and hot-reload the game's
/// [`game_interface_glue`] shared library as well as calling the libraries' provided functions
/// that are defined in the [`game_interface_glue`] crate.
///
/// [`game_interface_glue`]: ../../game_interface_glue/index.html
pub struct GameLib {
    pub lib: Library,
    lib_path: String,
    lib_name: String,
    last_modified_time: std::time::SystemTime,
    copy_counter: usize,
}

impl GameLib {
    /// Forwards to the dynamic libraries' corresponding `update_and_draw` function
    pub fn update_and_draw(&self, input: &GameInput, game_state: &mut GameContext) {
        unsafe {
            let f = self
                .lib
                .get::<fn(&GameInput, &mut GameContext)>(b"update_and_draw\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "Could not load `update_and_draw` function from GameLib: {}",
                        error
                    )
                });
            f(input, game_state)
        }
    }

    /// Forwards to the dynamic libraries' corresponding `get_audio_samples` function
    pub fn process_audio(&self, input: &GameInput, gc: &mut GameContext, ac: &mut AudioContext) {
        unsafe {
            let f = self
                .lib
                .get::<fn(&GameInput, &mut GameContext, &mut AudioContext)>(b"process_audio\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "Could not load `process_audio` function from GameLib: {}",
                        error
                    )
                });
            f(input, gc, ac)
        }
    }

    /// Makes a copy of the given dynamic library in a temporary directory and loads it. The copy
    /// is necessary to circumvent file locking issues on MS Windows.
    pub fn new(lib_path: &str, lib_name: &str) -> GameLib {
        GameLib::load(0, lib_path, lib_name)
    }

    /// Checks if the dynamic library has changed since we last (re-)loaded it and therefore
    /// needs to be reloaded. This internally compares the files `last modified` timestamp
    /// with the timestamp of the last (re-)loading of the library.
    pub fn needs_reloading(&mut self) -> bool {
        let (file_path, _, _) =
            GameLib::construct_paths(self.copy_counter, &self.lib_path, &self.lib_name);

        if let Ok(Ok(last_modified_time)) =
            std::fs::metadata(&file_path).map(|metadata| metadata.modified())
        {
            // NOTE: We do not set `self.last_modified_time` here because we might call this
            //       function multiple times and want the same result everytime until we reload
            last_modified_time > self.last_modified_time
        } else {
            false
        }
    }

    /// Reloads the dynamic library. Note this will reload the library even if it has not changed
    /// since the last reloading. To prevent this you can use the [`needs_reloading`] method first.
    ///
    /// [`needs_reloading`]: #method.needs_reloading
    pub fn reload(self) -> GameLib {
        let lib_path = self.lib_path.clone();
        let lib_name = self.lib_name.clone();
        let mut copy_counter = self.copy_counter;

        if GameLib::copy_lib(copy_counter, &lib_path, &lib_name).is_err() {
            // NOTE: It can happen (even multiple times) that we fail to copy the library while
            //       it is being recompiled/updated. This is OK as we can just retry the next time.
            // TODO(JaSc): Maybe we could implement a fail counter so that after i.e. 10 tries this
            //             will hard panic instead of just returning.
            return self;
        }

        copy_counter += 1;
        drop(self);
        GameLib::load(copy_counter, &lib_path, &lib_name)
    }

    fn load(mut copy_counter: usize, lib_path: &str, lib_name: &str) -> GameLib {
        GameLib::copy_lib(copy_counter, lib_path, lib_name)
            .unwrap_or_else(|error| panic!("Error while copying: {}", error));
        let (file_path, _, copy_file_path) =
            GameLib::construct_paths(copy_counter, lib_path, lib_name);
        copy_counter += 1;

        // NOTE: Loading from a copy is necessary on MS Windows due to write protection issues
        let lib = Library::new(&copy_file_path).unwrap_or_else(|error| {
            panic!("Failed to load library {} : {}", copy_file_path, error)
        });

        let last_modified_time = std::fs::metadata(&file_path)
            .unwrap_or_else(|error| {
                panic!("Cannot open file {} to read metadata: {}", file_path, error)
            })
            .modified()
            .unwrap_or_else(|error| {
                panic!("Cannot read metadata of file {}: {}", file_path, error)
            });

        info!("Game lib reloaded");
        GameLib {
            lib,
            lib_path: String::from(lib_path),
            lib_name: String::from(lib_name),
            last_modified_time,
            copy_counter,
        }
    }

    /// Creates a temporary folder (if necessary) in the libraries' root path  and copies our
    /// library into it
    fn copy_lib(
        copy_counter: usize,
        lib_path: &str,
        lib_name: &str,
    ) -> Result<u64, std::io::Error> {
        // Construct necessary file paths
        let (file_path, copy_path, copy_file_path) =
            GameLib::construct_paths(copy_counter, lib_path, lib_name);

        std::fs::create_dir_all(&copy_path)
            .unwrap_or_else(|error| panic!("Cannot create dir {}: {}", copy_path, error));

        // NOTE: Copy may fail while the library is being rebuild by cargo
        let copy_result = std::fs::copy(&file_path, &copy_file_path);
        if let Err(ref error) = copy_result {
            warn!(
                "Cannot copy file {} to {} right now. Please try again soon: {}",
                file_path, copy_file_path, error
            )
        }
        copy_result
    }

    fn construct_paths(
        copy_counter: usize,
        lib_path: &str,
        lib_name: &str,
    ) -> (String, String, String) {
        let file_path = String::from(lib_path) + &GameLib::lib_name_to_file_name(lib_name);
        let copy_path = String::from(lib_path) + "libcopies/";
        let copy_file_path = copy_path.clone()
            + &GameLib::lib_name_to_file_name(
                &(String::from(lib_name) + &copy_counter.to_string()),
            );

        (file_path, copy_path, copy_file_path)
    }

    #[cfg(target_os = "windows")]
    fn lib_name_to_file_name(lib_name: &str) -> String {
        format!("{}.dll", lib_name)
    }
    #[cfg(target_os = "linux")]
    fn lib_name_to_file_name(lib_name: &str) -> String {
        format!("lib{}.so", lib_name)
    }
}
