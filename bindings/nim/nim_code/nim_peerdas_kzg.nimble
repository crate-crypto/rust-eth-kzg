# Package

# x-release-please-start-version
version       = "0.3.0"
# x-release-please-end

author        = "Kevaundray Wedderburn"
description   = "PeerDas KZG bindings"
license       = "MIT"

import src/build_utils

const staticLibInstallDir = getInstallDir()

installDirs   = @[
  "src",
  staticLibInstallDir,
]

# Dependencies

requires "nim >= 2.0.4"
requires "yaml"
requires "unittest2"
requires "stew"
