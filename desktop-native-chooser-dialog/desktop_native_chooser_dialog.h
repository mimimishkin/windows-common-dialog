#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum ChoosingMode {
  Saving,
  FilesOnly,
  FilesAndDirectories,
  DirectoriesOnly,
} ChoosingMode;

typedef enum ChooserDialogOwner_Tag {
  HWND,
  X11,
  Wayland,
  NSWindow,
  Desc,
  None,
} ChooserDialogOwner_Tag;

typedef struct HWND_Body {
  void *ptr;
} HWND_Body;

typedef struct X11_Body {
  uint64_t window;
} X11_Body;

typedef struct Wayland_Body {
  void *surface_ptr;
  void *display_ptr;
} Wayland_Body;

typedef struct NSWindow_Body {
  const void *ptr;
} NSWindow_Body;

typedef struct Desc_Body {
  const char *title;
  int32_t x;
  int32_t y;
  int32_t width;
  int32_t height;
} Desc_Body;

typedef struct ChooserDialogOwner {
  ChooserDialogOwner_Tag tag;
  union {
    HWND_Body hwnd;
    X11_Body x11;
    Wayland_Body wayland;
    NSWindow_Body ns_window;
    Desc_Body desc;
  };
} ChooserDialogOwner;

typedef struct SizedStrPtr {
  const uint8_t *ptr;
  uintptr_t len;
} SizedStrPtr;

typedef struct FFIResult {
  bool is_ok;
  struct SizedStrPtr result_or_message;
} FFIResult;

/**
 * # Safety
 * * [title] must be either a valid pointer to utf8 string without a trailing null, or null.
 * * [title_size] must contain the size of [title] in bytes or -1.
 * * The same with [initial_directory] and [suggested_name].
 * * [filters] must be either a valid pointer to an array of utf8 strings without a trailing null
 *   terminated by '\0\0', or null.
 * * [filters_size] must contain the size of [filters] in bytes or -1.
 * * Caller takes care about freeing the result via [free_ffi_string].
 */
void choose_ffi(uint64_t id_part1,
                uint64_t id_part2,
                const unsigned char *title,
                int64_t title_size,
                const unsigned char *filters,
                int64_t filters_size,
                enum ChoosingMode mode,
                bool multiple,
                const unsigned char *initial_directory,
                int64_t initial_directory_size,
                const unsigned char *suggested_name,
                int64_t suggested_name_size,
                struct ChooserDialogOwner owner,
                void (*callback)(struct FFIResult));

/**
 * # Safety
 * Caller takes care about freeing the result via [free_ffi_string].
 */
struct FFIResult load_mime_table_ffi(void);

/**
 * # Safety
 * Caller takes care about freeing the result via [free_ffi_string].
 */
struct FFIResult load_folder_ffi(void);

/**
 * # Safety
 * [string] must contain a valid pointer obtained from Rust String.
 */
void free_ffi_string(struct SizedStrPtr string);

/**
 * Can be used to check if the library is available on the current platform.
 */
bool check_available(void);
