#include <stdbool.h>
#include <stdint.h>

// All calls, including event polling and block disposal, belong to the main thread.
// Returned strings are owned by the caller; release with ha_sparkle_free_string.
char *ha_sparkle_prototype_config(void);
void *ha_sparkle_create(void);
char *ha_sparkle_next_event(void *handle);
bool ha_sparkle_action(void *handle, uint32_t action, uint64_t cycle);
void ha_sparkle_free_string(char *value);
void ha_sparkle_destroy(void *handle);
