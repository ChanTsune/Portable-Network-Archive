#![cfg(windows)]

use std::fs::File;
use std::io::{self, Write, Read};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand; // Renamed to avoid conflict with assert_cmd::Command
use std::str;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

// Assuming utils are in the same test crate or accessible
// For PnaCommandExt etc.
mod utils;
use utils::{CommandExt, OutputExt, PnaCommandExt}; // Assuming these are defined in utils

// Helper function to set a file property using PowerShell's Set-ItemProperty
fn set_file_property_ps(path: &Path, property_name: &str, value: &str) -> io::Result<()> {
    // Ensure path is absolute for PowerShell
    let absolute_path = path.canonicalize()?;
    let ps_command = format!(
        "Set-ItemProperty -Path '{}' -Name '{}' -Value '{}'",
        absolute_path.display(),
        property_name,
        value
    );

    let status = StdCommand::new("powershell")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(&ps_command)
        .status()?;

    if status.success() {
        // Give a moment for the property system to update
        std::thread::sleep(std::time::Duration::from_millis(500));
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "PowerShell Set-ItemProperty failed for '{}' with status: {}. Command: {}",
                path.display(),
                status,
                ps_command
            ),
        ))
    }
}

// Helper function to get a file property using PowerShell's Get-ItemProperty
fn get_file_property_ps(path: &Path, property_name: &str) -> io::Result<Option<String>> {
    let absolute_path = path.canonicalize()?;
    let ps_command = format!(
        "(Get-ItemProperty -Path '{}' -Name '{}' -ErrorAction SilentlyContinue).{}",
        absolute_path.display(),
        property_name,
        property_name // To access the property directly from the output object
    );

    let output = StdCommand::new("powershell")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(&ps_command)
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("stdout not UTF-8: {}", e))
        })?;
        let value = stdout.trim();
        if value.is_empty() {
            Ok(None)
        } else {
            // PowerShell might return multi-line strings or arrays as space-separated.
            // For System.Author, it might be an array. If so, join with "; ".
            // For simplicity here, we assume single string or join if it looks like an array.
            // This might need refinement based on actual Get-ItemProperty output for arrays.
            if property_name == "System.Author" && value.contains('\n') {
                 Ok(Some(value.lines().map(|s| s.trim()).collect::<Vec<&str>>().join("; ")))
            } else {
                 Ok(Some(value.to_string()))
            }
        }
    } else {
        let stderr = String::from_utf8(output.stderr).unwrap_or_default();
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "PowerShell Get-ItemProperty failed for '{}', property '{}', with status: {}. Stderr: {}. Command: {}",
                path.display(),
                property_name,
                output.status,
                stderr,
                ps_command
            ),
        ))
    }
}

// Helper to create a dummy file
fn create_dummy_file(dir: &TempDir, name: &str, content: &[u8]) -> io::Result<PathBuf> {
    let file_path = dir.path().join(name);
    let mut file = File::create(&file_path)?;
    file.write_all(content)?;
    Ok(file_path)
}

#[test]
fn test_store_and_restore_title_property() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file_name = "title_test.txt";
    let test_file = create_dummy_file(&temp_dir, test_file_name, b"Title content")?;
    let expected_title = "PNA Test Document Title";

    set_file_property_ps(&test_file, "System.Title", expected_title)?;

    let archive_path = temp_dir.path().join("title_archive.pna");

    // Create archive
    StdCommand::pna()
        .arg("create")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--store-windows-properties")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Optional: Verify xattr
    let xattr_output = StdCommand::pna()
        .arg("xattr")
        .arg("get")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--name")
        .arg("pna.windows.property.System.Title")
        .current_dir(temp_dir.path())
        .output()?;
    assert!(xattr_output.status.success());
    let xattr_stdout = str::from_utf8(&xattr_output.stdout)?;
    assert!(xattr_stdout.contains(expected_title));
    
    let output_dir = temp_dir.path().join("output_title");
    fs::create_dir(&output_dir)?;

    // Extract archive
    StdCommand::pna()
        .arg("extract")
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&output_dir)
        .arg("--restore-windows-properties")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let extracted_file_path = output_dir.join(test_file_name);
    assert!(extracted_file_path.exists());

    let restored_title = get_file_property_ps(&extracted_file_path, "System.Title")?;
    assert_eq!(restored_title, Some(expected_title.to_string()));

    Ok(())
}

#[test]
fn test_store_and_restore_comment_property() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file_name = "comment_test.txt";
    let test_file = create_dummy_file(&temp_dir, test_file_name, b"Comment content")?;
    let expected_comment = "This is a test comment for PNA.";

    set_file_property_ps(&test_file, "System.Comment", expected_comment)?;

    let archive_path = temp_dir.path().join("comment_archive.pna");

    StdCommand::pna()
        .arg("create")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--store-windows-properties")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    
    let output_dir = temp_dir.path().join("output_comment");
    fs::create_dir(&output_dir)?;

    StdCommand::pna()
        .arg("extract")
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&output_dir)
        .arg("--restore-windows-properties")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let extracted_file_path = output_dir.join(test_file_name);
    let restored_comment = get_file_property_ps(&extracted_file_path, "System.Comment")?;
    assert_eq!(restored_comment, Some(expected_comment.to_string()));
    Ok(())
}

#[test]
fn test_store_and_restore_author_property_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file_name = "author_test.txt";
    let test_file = create_dummy_file(&temp_dir, test_file_name, b"Author content")?;
    // PowerShell Set-ItemProperty for System.Author takes a string array.
    // Here, we pass a single string that PowerShell will interpret.
    // If we wanted to pass an array from Rust to PS, it'd be more complex.
    // For this test, we set it as if it were a single string with semicolons.
    let authors_value_ps = "@('Author One','Author Two')"; // PowerShell array syntax
    let expected_authors_stored = "Author One; Author Two"; // How pna should store it (joined from array)
    let expected_authors_restored = "Author One; Author Two"; // How pna should restore it (as a single string)

    // Use a slightly different PowerShell command for setting array properties
    let absolute_path = test_file.canonicalize()?;
    let ps_command = format!(
        "Set-ItemProperty -Path '{}' -Name 'System.Author' -Value {}",
        absolute_path.display(),
        authors_value_ps 
    );
     StdCommand::new("powershell")
        .arg("-NoProfile").arg("-NonInteractive").arg("-Command").arg(&ps_command)
        .status()?.success();
    std::thread::sleep(std::time::Duration::from_millis(500));


    let archive_path = temp_dir.path().join("author_archive.pna");
    StdCommand::pna()
        .arg("create")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--store-windows-properties")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let xattr_output = StdCommand::pna()
        .arg("xattr")
        .arg("get")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--name")
        .arg("pna.windows.property.System.Author")
        .current_dir(temp_dir.path())
        .output()?;
    assert!(xattr_output.status.success());
    let xattr_stdout = str::from_utf8(&xattr_output.stdout)?;
    assert!(xattr_stdout.contains(expected_authors_stored));

    let output_dir = temp_dir.path().join("output_author");
    fs::create_dir(&output_dir)?;
    StdCommand::pna()
        .arg("extract")
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&output_dir)
        .arg("--restore-windows-properties")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let extracted_file_path = output_dir.join(test_file_name);
    let restored_authors = get_file_property_ps(&extracted_file_path, "System.Author")?;
    // PowerShell's Get-ItemProperty for System.Author returns an array, which our helper joins with "; "
    assert_eq!(restored_authors, Some(expected_authors_restored.to_string()));
    Ok(())
}

#[test]
fn test_extract_without_restore_flag_preserves_no_properties() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file_name = "no_restore_prop.txt";
    let test_file = create_dummy_file(&temp_dir, test_file_name, b"Content")?;
    let title_val = "Temporary Title";
    set_file_property_ps(&test_file, "System.Title", title_val)?;

    let archive_path = temp_dir.path().join("no_restore_archive.pna");
    StdCommand::pna()
        .arg("create")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--store-windows-properties")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let output_dir = temp_dir.path().join("output_no_restore_prop");
    fs::create_dir(&output_dir)?;
    StdCommand::pna()
        .arg("extract")
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&output_dir)
        // NO --restore-windows-properties flag
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let extracted_file_path = output_dir.join(test_file_name);
    let restored_title = get_file_property_ps(&extracted_file_path, "System.Title")?;
    // Should be None or empty, definitely not title_val
    assert_ne!(restored_title, Some(title_val.to_string()));
    Ok(())
}

#[test]
fn test_store_no_properties_if_not_set() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file_name = "no_prop_set.txt";
    create_dummy_file(&temp_dir, test_file_name, b"Plain content")?; // No properties set

    let archive_path = temp_dir.path().join("no_prop_archive.pna");
    StdCommand::pna()
        .arg("create")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--store-windows-properties") // Flag is present
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Check that specific property xattrs are NOT present
    StdCommand::pna()
        .arg("xattr")
        .arg("get")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--name")
        .arg("pna.windows.property.System.Title")
        .current_dir(temp_dir.path())
        .assert()
        .failure() // Expect failure because the xattr should not exist
        .stderr(predicate::str::contains("Extended attribute not found")); 

    StdCommand::pna()
        .arg("xattr")
        .arg("get")
        .arg(&archive_path)
        .arg(test_file_name)
        .arg("--name")
        .arg("pna.windows.property.System.Author")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Extended attribute not found"));
    Ok(())
}
