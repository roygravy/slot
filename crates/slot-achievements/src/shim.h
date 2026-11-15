// Small flat C API over rc_client so Rust never touches rcheevos structs.
// The game thread drives it; the network worker takes requests and hands their
// answers to rcheevos itself through ra_complete_request, so a big set is read
// there and not in the middle of a frame. The shim locks its own state.
#ifndef SLOT_RA_SHIM_H
#define SLOT_RA_SHIM_H
#include <stddef.h>
#include <stdint.h>

typedef struct ra ra_t;

typedef struct {
    uint32_t id;
    char *url;
    char *post;
    char *content_type;
} ra_request;

typedef struct {
    int kind; // 1 unlock, 2 game completed, 3 server error, 4 disconnected, 5 reconnected
    uint32_t id;
    uint32_t points;
    char title[128];
    char description[256];
    char badge[16];
    char message[256];
} ra_event;

typedef struct {
    int state; // 0 idle, 1 logging in, 2 logged in, 3 loading, 4 game ready, 5 unknown game, 6 failed
    uint32_t game_id;
    char game_title[96];
    char game_badge[16];
    char user[64];
    char error[256];
    uint32_t total;
    uint32_t unlocked;
    uint32_t points_total;
    uint32_t points_unlocked;
    int64_t beaten_time; // 0 if not beaten
} ra_info;

typedef struct {
    uint32_t id;
    uint32_t points;
    int unlocked;
    int64_t unlock_time;
    float rarity;
    char title[96];
    char description[256];
    char badge[16];
} ra_ach;

typedef void (*ra_core_memory_fn)(void *ud, uint32_t id, void **data, size_t *size);

ra_t *ra_new(void);
void ra_free(ra_t *);
// mmap: the core's retro_memory_map copy, or NULL to use the core's plain memory regions.
void ra_set_memory(ra_t *, uint32_t console_id, const void *mmap, ra_core_memory_fn info, void *ud);
void ra_login_token(ra_t *, const char *user, const char *token);
void ra_logout(ra_t *);
void ra_load_game(ra_t *, uint32_t console_id, const uint8_t *rom, size_t len);
void ra_unload_game(ra_t *);
/* The core is going away: no more memory reads. */
void ra_forget_memory(ra_t *);
void ra_do_frame(ra_t *);
void ra_idle(ra_t *);
int ra_take_request(ra_t *, ra_request *out);
void ra_free_request(ra_request *);
void ra_complete_request(ra_t *, uint32_t id, int status, const uint8_t *body, size_t len);
int ra_take_event(ra_t *, ra_event *out);
void ra_get_info(ra_t *, ra_info *out);
int ra_ach_count(ra_t *);
int ra_ach_get(ra_t *, int index, ra_ach *out);
void ra_refresh_list(ra_t *);
size_t ra_user_agent_clause(ra_t *, char *, size_t);
#endif
