#include <stdbool.h>
#include <stdint.h>

// All calls, including event polling and block disposal, belong to the main thread.
// Returned strings are owned by the caller; release with ha_sparkle_free_string.
char *ha_sparkle_config(void);
bool ha_sparkle_migration_needed(void);
typedef void (*HASparkleWake)(void);
void *ha_sparkle_create(int32_t legacy_auto_check, HASparkleWake wake);
bool ha_sparkle_set_auto_check(void *handle, bool value);
bool ha_sparkle_can_check(void *handle);
char *ha_sparkle_next_event(void *handle);
bool ha_sparkle_action(void *handle, uint32_t action, uint64_t cycle);
void ha_sparkle_free_string(char *value);
void ha_sparkle_destroy(void *handle);
