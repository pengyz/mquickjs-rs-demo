#
# Vela build system CMake module entry on top of Nuttx
#
# VENDORED OVERRIDE (ports/openvela/cmake/): this is a copy of
# build/cmake/nuttx_custom_module.cmake from the openvela build tree with
# the unconditional `include(apps/external/optee/TA*.cmake)` lines REMOVED.
#
# Why this exists: the port integration must not modify the openvela tree
# (public infrastructure). The stock module hard-includes optee cmake files
# that live in repos a partial `repo sync` does not fetch, which fails the
# cmake configure for this port (optee is not used here). We point
# -DCUSTOM_MODULE_PATH at this directory instead.
#
# Drift note: if the stock module gains new logic, re-copy it here and keep
# the optee includes removed (or make them conditional on the files existing).
#

# Vela build system custom modules — optee stubbed out for this port.
# (TA.cmake / TA_lib.cmake intentionally not included; add here only if a
#  future integration actually needs optee TAs.)
