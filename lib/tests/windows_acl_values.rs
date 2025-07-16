#![cfg(windows)]

use libpna::AcePermission;
use windows::Win32::Storage::FileSystem::{
    DELETE, FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_APPEND_DATA, FILE_DELETE_CHILD,
    FILE_EXECUTE, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_READ_EA,
    FILE_TRAVERSE, FILE_WRITE_ATTRIBUTES, FILE_WRITE_DATA, FILE_WRITE_EA, READ_CONTROL,
    SYNCHRONIZE, WRITE_DAC, WRITE_OWNER,
};

#[test]
fn windows_acl_constants_match_winnt() {
    // Specific rights
    assert_eq!(
        AcePermission::WINDOWS_READ_DATA.bits(),
        FILE_READ_DATA.0 as u32
    );
    assert_eq!(
        AcePermission::WINDOWS_LIST_DIRECTORY.bits(),
        FILE_LIST_DIRECTORY.0 as u32
    );
    assert_eq!(
        AcePermission::WINDOWS_WRITE_DATA.bits(),
        FILE_WRITE_DATA.0 as u32
    );
    assert_eq!(
        AcePermission::WINDOWS_ADD_FILE.bits(),
        FILE_ADD_FILE.0 as u32
    );
    assert_eq!(
        AcePermission::WINDOWS_APPEND_DATA.bits(),
        FILE_APPEND_DATA.0 as u32
    );
    assert_eq!(
        AcePermission::WINDOWS_ADD_SUBDIRECTORY.bits(),
        FILE_ADD_SUBDIRECTORY.0 as u32
    );
    assert_eq!(AcePermission::WINDOWS_READ_EA.bits(), FILE_READ_EA.0 as u32);
    assert_eq!(
        AcePermission::WINDOWS_WRITE_EA.bits(),
        FILE_WRITE_EA.0 as u32
    );
    assert_eq!(AcePermission::WINDOWS_EXECUTE.bits(), FILE_EXECUTE.0 as u32);
    assert_eq!(
        AcePermission::WINDOWS_TRAVERSE.bits(),
        FILE_TRAVERSE.0 as u32
    );
    assert_eq!(
        AcePermission::WINDOWS_DELETE_CHILD.bits(),
        FILE_DELETE_CHILD.0 as u32
    );
    assert_eq!(
        AcePermission::WINDOWS_READ_ATTRIBUTES.bits(),
        FILE_READ_ATTRIBUTES.0 as u32
    );
    assert_eq!(
        AcePermission::WINDOWS_WRITE_ATTRIBUTES.bits(),
        FILE_WRITE_ATTRIBUTES.0 as u32
    );

    // Standard rights
    assert_eq!(AcePermission::WINDOWS_DELETE.bits(), DELETE.0 as u32);
    assert_eq!(
        AcePermission::WINDOWS_READ_CONTROL.bits(),
        READ_CONTROL.0 as u32
    );
    assert_eq!(AcePermission::WINDOWS_WRITE_DAC.bits(), WRITE_DAC.0 as u32);
    assert_eq!(
        AcePermission::WINDOWS_WRITE_OWNER.bits(),
        WRITE_OWNER.0 as u32
    );
    assert_eq!(
        AcePermission::WINDOWS_SYNCHRONIZE.bits(),
        SYNCHRONIZE.0 as u32
    );
}
