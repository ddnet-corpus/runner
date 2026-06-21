use colored::Colorize;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsStr,
    fmt::Display,
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

#[derive(Error, Debug)]
pub(super) enum NewCorpusError {
    #[error(transparent)]
    WalkDir(#[from] walkdir::Error),
    #[error(
        "File '{}' has invalid role '{}', valid roles are 'in' and 'expected'",
        file_path,
        role
    )]
    InvalidFileRole { file_path: String, role: String },
    #[error("Missing {} file for test '{}' at '{}'", role, test, path.display())]
    MissingRoleFile {
        role: FileRole,
        test: String,
        path: PathBuf,
    },
}

#[derive(Debug)]
pub(super) enum FileRole {
    Input,
    Expected,
}

impl FileRole {
    fn extension(&self) -> &'static str {
        match self {
            Self::Input => "in",
            Self::Expected => "expected",
        }
    }
}

impl Display for FileRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input => write!(f, "input"),
            Self::Expected => write!(f, "expected"),
        }
    }
}

#[derive(Debug)]
pub(super) struct Test {
    path: PathBuf,
    input_ext: String,
    expected_ext: String,
}

type Matcher = fn(&[u8], &[u8]) -> Result<(), String>;
type Runner = Box<dyn Fn(&[u8], &str) -> Vec<u8>>;

#[derive(Debug, Error)]
pub(super) enum RunCorpusError<'a> {
    #[error("Matcher not found for extension '{}'", _0)]
    MatcherNotFound(&'a str),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub(super) struct Corpus<'a> {
    root: &'a Path,
    tests: Vec<Test>,
    matchers: HashMap<String, Matcher>,
    runners: BTreeMap<PathBuf, Runner>,
}

impl<'a> Corpus<'a> {
    pub(super) fn new(root: &'a Path) -> Result<Self, NewCorpusError> {
        let mut tests: BTreeMap<PathBuf, (Option<String>, Option<String>)> = BTreeMap::new();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
        {
            let entry = entry?;

            if entry.file_type().is_dir() {
                continue;
            }

            let (name, role, ext) = match parse_file_name(entry.file_name()) {
                Ok(values) => values,
                Err(err) => match err {
                    ParseFileError::InvalidFileName => continue,
                    ParseFileError::InvalidFileRole(role) => {
                        return Err(NewCorpusError::InvalidFileRole {
                            file_path: entry.path().to_string_lossy().to_string(),
                            role,
                        });
                    }
                },
            };
            let path = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_path_buf()
                .with_file_name(name);
            let entry = tests.entry(path).or_default();

            match role {
                FileRole::Input => entry.0 = Some(ext),
                FileRole::Expected => entry.1 = Some(ext),
            }
        }

        let tests = tests
            .into_iter()
            .map(|(path, exts)| match exts {
                (Some(input), Some(expected)) => Ok(Test {
                    path,
                    input_ext: input,
                    expected_ext: expected,
                }),
                (Some(_), None) => Err(NewCorpusError::MissingRoleFile {
                    role: FileRole::Expected,
                    test: path.file_name().unwrap().to_string_lossy().to_string(),
                    path: path.parent().unwrap().into(),
                }),
                (None, Some(_)) => Err(NewCorpusError::MissingRoleFile {
                    role: FileRole::Input,
                    test: path.file_name().unwrap().to_string_lossy().to_string(),
                    path: path.parent().unwrap().into(),
                }),
                (None, None) => unreachable!(),
            })
            .collect::<Result<_, _>>()?;

        Ok(Self {
            root,
            tests,
            matchers: HashMap::new(),
            runners: BTreeMap::new(),
        })
    }

    pub(super) fn add_matcher(&mut self, ext: String, matcher: Matcher) {
        assert!(
            self.matchers.insert(ext.clone(), matcher).is_none(),
            "matcher is already defined for extension '{}'",
            ext,
        );
    }

    pub(super) fn add_runner(&mut self, path: PathBuf, runner: Runner) {
        assert!(
            self.tests.iter().any(|t| t.path.starts_with(&path)),
            "path '{}' doesn't match any test",
            path.display()
        );
        assert!(
            self.runners.insert(path.clone(), runner).is_none(),
            "runner is already defined for path '{}'",
            path.display(),
        );
    }

    pub(super) fn find_runner(&self, path: &Path) -> Option<&Runner> {
        self.runners
            .iter()
            .find_map(|(p, runner)| path.starts_with(p).then_some(runner))
    }

    pub(super) fn run(&self) -> Result<bool, RunCorpusError<'_>> {
        fn file_path(root: &Path, path: &Path, role: FileRole, ext: &str) -> PathBuf {
            root.join(path)
                .with_added_extension(role.extension())
                .with_added_extension(ext)
        }

        let mut all_passed = true;

        for Test {
            path,
            input_ext,
            expected_ext,
        } in &self.tests
        {
            let Some(runner) = self.find_runner(path) else {
                println!("{} - {}", path.display(), "skipped".cyan());

                continue;
            };
            let Some(matcher) = self.matchers.get(expected_ext) else {
                return Err(RunCorpusError::MatcherNotFound(expected_ext));
            };
            let input = fs::read(file_path(self.root, path, FileRole::Input, input_ext))?;
            let expected = fs::read(file_path(self.root, path, FileRole::Expected, expected_ext))?;
            let actual = runner(&input, input_ext);

            print!("{} - ", path.display());

            match matcher(&actual, &expected) {
                Ok(()) => {
                    println!("{}", "pass".green());
                }
                Err(message) => {
                    all_passed = false;

                    println!("{}", "fail".red());
                    println!("{}", message);
                }
            }
        }

        Ok(all_passed)
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

enum ParseFileError {
    InvalidFileName,
    InvalidFileRole(String),
}

fn parse_file_name(file_name: &OsStr) -> Result<(String, FileRole, String), ParseFileError> {
    let [ext, role, name]: [_; 3] = file_name
        .to_string_lossy()
        .to_string()
        .rsplitn(3, '.')
        .map(str::to_string)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| ParseFileError::InvalidFileName)?;
    let role = match role.as_str() {
        "in" => FileRole::Input,
        "expected" => FileRole::Expected,
        _ => return Err(ParseFileError::InvalidFileRole(role)),
    };

    Ok((name, role, ext))
}
