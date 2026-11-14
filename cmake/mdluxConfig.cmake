# Optional helper for the manual integration path.
# Pass MDLUX_ROOT=/path/to/mdlux-rs to CMake.

if(NOT MDLUX_ROOT)
  message(FATAL_ERROR "MDLUX_ROOT must point to mdlux-rs")
endif()

if(NOT TARGET mdlux::mdlux)
  add_library(mdlux::mdlux STATIC IMPORTED)
  if(WIN32)
    set(MDLUX_ARCHIVE "mdlux.lib")
  else()
    set(MDLUX_ARCHIVE "libmdlux.a")
  endif()
  if(EXISTS "${MDLUX_ROOT}/${MDLUX_ARCHIVE}")
    set(MDLUX_IMPORT "${MDLUX_ROOT}/${MDLUX_ARCHIVE}")
  else()
    set(MDLUX_IMPORT "${MDLUX_ROOT}/target/release/${MDLUX_ARCHIVE}")
  endif()
  if(NOT EXISTS "${MDLUX_IMPORT}")
    message(FATAL_ERROR "mdlux static library not found: ${MDLUX_IMPORT}")
  endif()
  set_target_properties(mdlux::mdlux PROPERTIES
    IMPORTED_LOCATION "${MDLUX_IMPORT}"
    INTERFACE_INCLUDE_DIRECTORIES "${MDLUX_ROOT}/include")

  if(APPLE)
    target_link_libraries(mdlux::mdlux INTERFACE "-framework Security" "-framework CoreFoundation")
  elseif(UNIX)
    target_link_libraries(mdlux::mdlux INTERFACE m pthread dl)
  endif()
endif()
