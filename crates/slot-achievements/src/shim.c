// Flat C wrapper over rcheevos' rc_client. See shim.h for the threading rules.
#include "shim.h"

#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "rc_client.h"
#include "rc_consoles.h"
#include "rc_error.h"
#include "rc_libretro.h"

typedef struct pending {
    uint32_t id;
    char *url;
    char *post;
    char *ctype;
    int taken;
    rc_client_server_callback_t cb;
    void *cbdata;
    struct pending *next;
} pending;

#define EVENTS 16

struct ra {
    rc_client_t *client;
    rc_libretro_memory_regions_t regions;
    int regions_ready;
    pending *head;
    uint32_t next_id;
    ra_event events[EVENTS];
    int ev_head;
    int ev_len;
    ra_info info;
    ra_ach *list;
    int list_len;
    /* Answers are handed to rcheevos on the network thread (a big set took 2.4 s to read,
     * the game stood still), so the shim's own state is shared: this guards the requests,
     * events, info and list. Never held while calling into rc_client. */
    pthread_mutex_t lock;
};

#define LOCK(ra) pthread_mutex_lock(&(ra)->lock)
#define UNLOCK(ra) pthread_mutex_unlock(&(ra)->lock)

static ra_core_memory_fn g_core_info; // only set while rc_libretro_memory_init calls back synchronously
static void *g_core_ud;

static void copy(char *dst, size_t cap, const char *src) {
    if (!src) {
        dst[0] = 0;
        return;
    }
    snprintf(dst, cap, "%s", src);
}

static char *dupstr(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

static void core_info(uint32_t id, rc_libretro_core_memory_info_t *info) {
    void *data = NULL;
    size_t size = 0;
    if (g_core_info) g_core_info(g_core_ud, id, &data, &size);
    info->data = data;
    info->size = size;
}

static uint32_t read_memory(uint32_t address, uint8_t *buffer, uint32_t n, rc_client_t *client) {
    ra_t *ra = rc_client_get_userdata(client);
    if (!ra || !ra->regions_ready) return 0;
    return rc_libretro_memory_read(&ra->regions, address, buffer, n);
}

static void server_call(const rc_api_request_t *req, rc_client_server_callback_t cb, void *cbdata,
                        rc_client_t *client) {
    ra_t *ra = rc_client_get_userdata(client);
    pending *p = calloc(1, sizeof *p);
    if (!p) return;
    p->url = dupstr(req->url);
    p->post = dupstr(req->post_data);
    p->ctype = dupstr(req->content_type);
    p->cb = cb;
    p->cbdata = cbdata;
    LOCK(ra);
    p->id = ++ra->next_id;
    pending **tail = &ra->head;
    while (*tail) tail = &(*tail)->next;
    *tail = p;
    UNLOCK(ra);
}

static void push_event(ra_t *ra, const ra_event *e) {
    LOCK(ra);
    if (ra->ev_len == EVENTS) {
        ra->ev_head = (ra->ev_head + 1) % EVENTS;
        ra->ev_len--;
    }
    ra->events[(ra->ev_head + ra->ev_len) % EVENTS] = *e;
    ra->ev_len++;
    UNLOCK(ra);
}

static void on_event(const rc_client_event_t *e, rc_client_t *client) {
    ra_t *ra = rc_client_get_userdata(client);
    ra_event ev;
    memset(&ev, 0, sizeof ev);
    switch (e->type) {
    case RC_CLIENT_EVENT_ACHIEVEMENT_TRIGGERED:
        if (e->achievement && e->achievement->id >= 101000001u) return; // hardcore warning
        ev.kind = 1;
        if (e->achievement) {
            ev.id = e->achievement->id;
            ev.points = e->achievement->points;
            copy(ev.title, sizeof ev.title, e->achievement->title);
            copy(ev.description, sizeof ev.description, e->achievement->description);
            copy(ev.badge, sizeof ev.badge, e->achievement->badge_name);
        }
        break;
    case RC_CLIENT_EVENT_GAME_COMPLETED:
        ev.kind = 2;
        break;
    case RC_CLIENT_EVENT_SERVER_ERROR:
        ev.kind = 3;
        if (e->server_error) copy(ev.message, sizeof ev.message, e->server_error->error_message);
        break;
    case RC_CLIENT_EVENT_DISCONNECTED:
        ev.kind = 4;
        break;
    case RC_CLIENT_EVENT_RECONNECTED:
        ev.kind = 5;
        break;
    default:
        return;
    }
    push_event(ra, &ev);
}

ra_t *ra_new(void) {
    ra_t *ra = calloc(1, sizeof *ra);
    if (!ra) return NULL;
    pthread_mutex_init(&ra->lock, NULL);
    ra->client = rc_client_create(read_memory, server_call);
    if (!ra->client) {
        pthread_mutex_destroy(&ra->lock);
        free(ra);
        return NULL;
    }
    rc_client_set_userdata(ra->client, ra);
    rc_client_set_event_handler(ra->client, on_event);
    // Cheats and save states are fine: slot only does softcore.
    rc_client_set_hardcore_enabled(ra->client, 0);
    return ra;
}

// "rcheevos/x.y" for the User-Agent, so the server knows which integration talks to it.
size_t ra_user_agent_clause(ra_t *ra, char *buf, size_t n) {
    return rc_client_get_user_agent_clause(ra->client, buf, n);
}

static void drop_requests(ra_t *ra) {
    while (ra->head) {
        pending *p = ra->head;
        ra->head = p->next;
        free(p->url);
        free(p->post);
        free(p->ctype);
        free(p);
    }
}

void ra_free(ra_t *ra) {
    if (!ra) return;
    if (ra->regions_ready) rc_libretro_memory_destroy(&ra->regions);
    rc_client_destroy(ra->client);
    drop_requests(ra);
    free(ra->list);
    pthread_mutex_destroy(&ra->lock);
    free(ra);
}

void ra_set_memory(ra_t *ra, uint32_t console_id, const void *mmap, ra_core_memory_fn info, void *ud) {
    if (ra->regions_ready) {
        rc_libretro_memory_destroy(&ra->regions);
        ra->regions_ready = 0;
    }
    g_core_info = info;
    g_core_ud = ud;
    memset(&ra->regions, 0, sizeof ra->regions);
    ra->regions_ready =
        rc_libretro_memory_init(&ra->regions, (const struct retro_memory_map *)mmap, core_info, console_id) != 0;
    g_core_info = NULL;
    g_core_ud = NULL;
}

static void login_done(int result, const char *error, rc_client_t *client, void *ud) {
    ra_t *ra = ud;
    const rc_client_user_t *u = result == RC_OK ? rc_client_get_user_info(client) : NULL;
    LOCK(ra);
    if (result == RC_OK) {
        copy(ra->info.user, sizeof ra->info.user, u ? u->display_name : "");
        ra->info.state = 2;
        ra->info.error[0] = 0;
    } else {
        ra->info.state = 6;
        copy(ra->info.error, sizeof ra->info.error, error);
    }
    UNLOCK(ra);
}

void ra_login_token(ra_t *ra, const char *user, const char *token) {
    LOCK(ra);
    ra->info.state = 1;
    UNLOCK(ra);
    rc_client_begin_login_with_token(ra->client, user, token, login_done, ra);
}

void ra_logout(ra_t *ra) {
    rc_client_logout(ra->client);
    LOCK(ra);
    memset(&ra->info, 0, sizeof ra->info);
    free(ra->list);
    ra->list = NULL;
    ra->list_len = 0;
    UNLOCK(ra);
}

/* rc_client hands the list out in buckets (unlocked, locked, ...). Every achievement points
 * into its set's own array, in the order the set's makers gave them: sorted by address, each
 * keeps its place in the set whether earned or not. */
static int by_place(const void *a, const void *b) {
    uintptr_t x = (uintptr_t)*(const rc_client_achievement_t *const *)a;
    uintptr_t y = (uintptr_t)*(const rc_client_achievement_t *const *)b;
    return x < y ? -1 : x > y;
}

void ra_refresh_list(ra_t *ra) {
    ra_ach *list = NULL;
    int list_len = 0;
    rc_client_achievement_list_t *l = rc_client_create_achievement_list(
        ra->client, RC_CLIENT_ACHIEVEMENT_CATEGORY_CORE, RC_CLIENT_ACHIEVEMENT_LIST_GROUPING_LOCK_STATE);
    if (!l) {
        LOCK(ra);
        free(ra->list);
        ra->list = NULL;
        ra->list_len = 0;
        UNLOCK(ra);
        return;
    }
    int total = 0;
    for (uint32_t b = 0; b < l->num_buckets; b++) total += (int)l->buckets[b].num_achievements;
    const rc_client_achievement_t **all = calloc(total ? total : 1, sizeof *all);
    list = calloc(total ? total : 1, sizeof(ra_ach));
    if (list && all) {
        int n = 0;
        for (uint32_t b = 0; b < l->num_buckets; b++)
            for (uint32_t i = 0; i < l->buckets[b].num_achievements; i++)
                all[n++] = l->buckets[b].achievements[i];
        qsort(all, n, sizeof *all, by_place);
        for (int k = 0; k < n; k++) {
            const rc_client_achievement_t *a = all[k];
            // A list can name one twice (recently unlocked and unlocked): once is enough.
            if (k > 0 && a == all[k - 1]) continue;
            // The server's "Unknown Emulator" warning is about hardcore, which slot never plays.
            if (a->id >= 101000001u) continue;
            ra_ach *o = &list[list_len++];
            o->id = a->id;
            o->points = a->points;
            o->unlocked = a->unlocked != 0;
            o->unlock_time = (int64_t)a->unlock_time;
            o->rarity = a->rarity;
            copy(o->title, sizeof o->title, a->title);
            copy(o->description, sizeof o->description, a->description);
            copy(o->badge, sizeof o->badge, a->badge_name);
        }
    }
    free(all);
    rc_client_destroy_achievement_list(l);
    LOCK(ra);
    free(ra->list);
    ra->list = list;
    ra->list_len = list_len;
    UNLOCK(ra);
}

static void load_done(int result, const char *error, rc_client_t *client, void *ud) {
    ra_t *ra = ud;
    const rc_client_game_t *g = rc_client_get_game_info(client);
    if (result == RC_OK && g) {
        rc_client_user_game_summary_t s;
        rc_client_get_user_game_summary(client, &s);
        ra_refresh_list(ra);
        LOCK(ra);
        ra->info.state = 4;
        ra->info.game_id = g->id;
        copy(ra->info.game_title, sizeof ra->info.game_title, g->title);
        copy(ra->info.game_badge, sizeof ra->info.game_badge, g->badge_name);
        ra->info.total = s.num_core_achievements;
        ra->info.unlocked = s.num_unlocked_achievements;
        ra->info.points_total = s.points_core;
        ra->info.points_unlocked = s.points_unlocked;
        ra->info.beaten_time = (int64_t)s.beaten_time;
        UNLOCK(ra);
    } else {
        LOCK(ra);
        ra->info.state = (g && g->id == 0) || result == RC_NO_GAME_LOADED ? 5 : 6;
        copy(ra->info.error, sizeof ra->info.error, error);
        UNLOCK(ra);
    }
}

void ra_load_game(ra_t *ra, uint32_t console_id, const uint8_t *rom, size_t len) {
    LOCK(ra);
    ra->info.state = 3;
    ra->info.game_id = 0;
    ra->info.total = ra->info.unlocked = ra->info.points_total = ra->info.points_unlocked = 0;
    ra->info.game_title[0] = 0;
    ra->info.error[0] = 0;
    UNLOCK(ra);
    rc_client_begin_identify_and_load_game(ra->client, console_id, NULL, rom, len, load_done, ra);
}

void ra_unload_game(ra_t *ra) {
    rc_client_unload_game(ra->client);
    LOCK(ra);
    ra->info.state = ra->info.user[0] ? 2 : 0;
    ra->info.game_id = 0;
    free(ra->list);
    ra->list = NULL;
    ra->list_len = 0;
    UNLOCK(ra);
}

void ra_forget_memory(ra_t *ra) {
    if (ra->regions_ready) {
        ra->regions_ready = 0;
        rc_libretro_memory_destroy(&ra->regions);
    }
}

void ra_do_frame(ra_t *ra) { rc_client_do_frame(ra->client); }
void ra_idle(ra_t *ra) { rc_client_idle(ra->client); }

int ra_take_request(ra_t *ra, ra_request *out) {
    LOCK(ra);
    for (pending *p = ra->head; p; p = p->next) {
        if (p->taken) continue;
        p->taken = 1;
        out->id = p->id;
        out->url = p->url;
        out->post = p->post;
        out->content_type = p->ctype;
        p->url = p->post = p->ctype = NULL;
        UNLOCK(ra);
        return 1;
    }
    UNLOCK(ra);
    return 0;
}

void ra_free_request(ra_request *r) {
    free(r->url);
    free(r->post);
    free(r->content_type);
    r->url = r->post = r->content_type = NULL;
}

void ra_complete_request(ra_t *ra, uint32_t id, int status, const uint8_t *body, size_t len) {
    LOCK(ra);
    pending **link = &ra->head;
    while (*link && (*link)->id != id) link = &(*link)->next;
    pending *p = *link;
    if (p) *link = p->next;
    UNLOCK(ra);
    if (!p) return;
    char *copy_body = malloc(len + 1);
    if (copy_body) {
        if (len) memcpy(copy_body, body, len);
        copy_body[len] = 0;
    }
    rc_api_server_response_t r;
    memset(&r, 0, sizeof r);
    r.body = copy_body;
    r.body_length = copy_body ? len : 0;
    r.http_status_code = status;
    p->cb(&r, p->cbdata);
    free(copy_body);
    free(p->url);
    free(p->post);
    free(p->ctype);
    free(p);
}

int ra_take_event(ra_t *ra, ra_event *out) {
    LOCK(ra);
    if (!ra->ev_len) {
        UNLOCK(ra);
        return 0;
    }
    *out = ra->events[ra->ev_head];
    ra->ev_head = (ra->ev_head + 1) % EVENTS;
    ra->ev_len--;
    UNLOCK(ra);
    return 1;
}

void ra_get_info(ra_t *ra, ra_info *out) {
    LOCK(ra);
    int ready = ra->info.state == 4;
    UNLOCK(ra);
    rc_client_user_game_summary_t s;
    if (ready) rc_client_get_user_game_summary(ra->client, &s);
    LOCK(ra);
    if (ready && ra->info.state == 4) {
        ra->info.total = s.num_core_achievements;
        ra->info.unlocked = s.num_unlocked_achievements;
        ra->info.points_total = s.points_core;
        ra->info.points_unlocked = s.points_unlocked;
        ra->info.beaten_time = (int64_t)s.beaten_time;
    }
    *out = ra->info;
    UNLOCK(ra);
}

int ra_ach_count(ra_t *ra) {
    LOCK(ra);
    int n = ra->list_len;
    UNLOCK(ra);
    return n;
}

int ra_ach_get(ra_t *ra, int index, ra_ach *out) {
    LOCK(ra);
    int ok = index >= 0 && index < ra->list_len;
    if (ok) *out = ra->list[index];
    UNLOCK(ra);
    return ok;
}
