# Package

# x-release-please-start-version
version       = "0.3.0"
# x-release-please-end

author        = "Kevaundray Wedderburn"
description   = "PeerDas KZG bindings"
license       = "MIT"
srcDir        = "src"

import src/utils
binDir = getInstallDir()

# Dependencies

requires "nim >= 2.0.4"
