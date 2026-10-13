use crate::ast::*;
use crate::diag::{Diag, Span};
use std::collections::{HashMap, HashSet};

const PRELUDE: &str = r##"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>
#include <math.h>
#include <time.h>

#if defined(IKS_PLATFORM_WINDOWS)
#define IKS_WIN 1
#elif defined(IKS_PLATFORM_LINUX)
#define IKS_LINUX 1
#elif defined(IKS_PLATFORM_ANDROID)
#define IKS_ANDROID 1
#elif defined(__ANDROID__)
#define IKS_ANDROID 1
#elif defined(_WIN32) || defined(__CYGWIN__)
#define IKS_WIN 1
#elif defined(__linux__)
#define IKS_LINUX 1
#else
#define IKS_OTHER 1
#endif

#if defined(IKS_WIN)
#include <windows.h>
#include <GL/gl.h>
#define COBJMACROS
#include <initguid.h>
#include <d3d9.h>
#include <d3d10.h>
#include <d3d11.h>
#include <d3d12.h>
#include <dxgi1_4.h>
#include <dxgi.h>
#include <d3dcompiler.h>
#include <pdh.h>
#endif

typedef struct { int64_t len; int64_t elemsize; int tag; void* data; } IksArray;

enum { IKS_T_INT = 0, IKS_T_FLOAT = 1, IKS_T_BOOL = 2, IKS_T_STR = 3, IKS_T_ARR = 4 };

static void iks_fail(const char* msg) {
    fprintf(stderr, "runtime error: %s\n", msg);
    exit(1);
}

static void iks_assert(int cond, const char* msg) {
    if (!cond) {
        fprintf(stderr, "assertion failed: %s\n", msg ? msg : "(no message)");
        exit(1);
    }
}

static void iks_beep(int64_t freq, int64_t ms) {
#if defined(IKS_WIN)
    Beep((DWORD)freq, (DWORD)ms);
#else
    (void)freq; (void)ms;
#endif
}

static IksArray iks_arr(int64_t elemsize, int64_t len, void* data, int tag) {
    IksArray a;
    a.len = len;
    a.elemsize = elemsize;
    a.tag = tag;
    a.data = NULL;
    if (len > 0) {
        a.data = malloc((size_t)(elemsize * len));
        if (!a.data) iks_fail("out of memory");
        if (data) memcpy(a.data, data, (size_t)(elemsize * len));
    }
    return a;
}

static void* iks_idx(IksArray a, int64_t i) {
    if (i < 0 || i >= a.len) {
        char buf[128];
        snprintf(buf, sizeof(buf), "index %lld out of bounds (array length is %lld)",
                 (long long)i, (long long)a.len);
        iks_fail(buf);
    }
    return (char*)a.data + i * a.elemsize;
}

static char* iks_cat(const char* a, const char* b) {
    size_t la = strlen(a), lb = strlen(b);
    char* s = (char*)malloc(la + lb + 1);
    if (!s) iks_fail("out of memory");
    memcpy(s, a, la);
    memcpy(s + la, b, lb);
    s[la + lb] = '\0';
    return s;
}

static int64_t iks_div_i(int64_t a, int64_t b) {
    if (b == 0) iks_fail("division by zero");
    return a / b;
}

static int64_t iks_mod_i(int64_t a, int64_t b) {
    if (b == 0) iks_fail("modulo by zero");
    return a % b;
}

static char* iks_str_i(int64_t v) {
    char* s = (char*)malloc(32);
    if (!s) iks_fail("out of memory");
    snprintf(s, 32, "%lld", (long long)v);
    return s;
}

static char* iks_str_f(double v) {
    char* s = (char*)malloc(64);
    if (!s) iks_fail("out of memory");
    snprintf(s, 64, "%.15g", v);
    return s;
}

static char* iks_str_b(bool v) {
    const char* t = v ? "true" : "false";
    size_t n = strlen(t);
    char* s = (char*)malloc(n + 1);
    if (!s) iks_fail("out of memory");
    memcpy(s, t, n + 1);
    return s;
}

/* ---- vec2 / vec3 / vec4 ---- */
typedef struct { double x, y; } IksVec2;
typedef struct { double x, y, z; } IksVec3;
typedef struct { double x, y, z, w; } IksVec4;

static IksVec2 iks_v2_add(IksVec2 a, IksVec2 b) { IksVec2 r; r.x = a.x + b.x; r.y = a.y + b.y; return r; }
static IksVec2 iks_v2_sub(IksVec2 a, IksVec2 b) { IksVec2 r; r.x = a.x - b.x; r.y = a.y - b.y; return r; }
static IksVec2 iks_v2_mul(IksVec2 a, IksVec2 b) { IksVec2 r; r.x = a.x * b.x; r.y = a.y * b.y; return r; }
static IksVec2 iks_v2_div(IksVec2 a, IksVec2 b) { IksVec2 r; r.x = a.x / b.x; r.y = a.y / b.y; return r; }
static IksVec2 iks_v2_scale(IksVec2 a, double s) { IksVec2 r; r.x = a.x * s; r.y = a.y * s; return r; }
static IksVec2 iks_v2_adds(IksVec2 a, double s) { IksVec2 r; r.x = a.x + s; r.y = a.y + s; return r; }
static IksVec2 iks_v2_subs(IksVec2 a, double s) { IksVec2 r; r.x = a.x - s; r.y = a.y - s; return r; }
static IksVec2 iks_v2_divs(IksVec2 a, double s) { IksVec2 r; r.x = a.x / s; r.y = a.y / s; return r; }
static double iks_v2_dot(IksVec2 a, IksVec2 b) { return a.x * b.x + a.y * b.y; }
static double iks_v2_len(IksVec2 a) { return sqrt(a.x * a.x + a.y * a.y); }
static IksVec2 iks_v2_norm(IksVec2 a) { IksVec2 r; double l = iks_v2_len(a); r.x = l > 0.0 ? a.x / l : 0.0; r.y = l > 0.0 ? a.y / l : 0.0; return r; }
static IksVec2 iks_v2_lerp(IksVec2 a, IksVec2 b, double t) { IksVec2 r; r.x = a.x + (b.x - a.x) * t; r.y = a.y + (b.y - a.y) * t; return r; }
static IksVec2 iks_v2_rot(IksVec2 a, double ang) { IksVec2 r; double c = cos(ang), s = sin(ang); r.x = a.x * c - a.y * s; r.y = a.x * s + a.y * c; return r; }
static char* iks_v2_str(IksVec2 a) { char* s = (char*)malloc(96); if (!s) iks_fail("out of memory"); snprintf(s, 96, "vec2(%.15g, %.15g)", a.x, a.y); return s; }

static IksVec3 iks_v3_add(IksVec3 a, IksVec3 b) { IksVec3 r; r.x = a.x + b.x; r.y = a.y + b.y; r.z = a.z + b.z; return r; }
static IksVec3 iks_v3_sub(IksVec3 a, IksVec3 b) { IksVec3 r; r.x = a.x - b.x; r.y = a.y - b.y; r.z = a.z - b.z; return r; }
static IksVec3 iks_v3_mul(IksVec3 a, IksVec3 b) { IksVec3 r; r.x = a.x * b.x; r.y = a.y * b.y; r.z = a.z * b.z; return r; }
static IksVec3 iks_v3_div(IksVec3 a, IksVec3 b) { IksVec3 r; r.x = a.x / b.x; r.y = a.y / b.y; r.z = a.z / b.z; return r; }
static IksVec3 iks_v3_scale(IksVec3 a, double s) { IksVec3 r; r.x = a.x * s; r.y = a.y * s; r.z = a.z * s; return r; }
static IksVec3 iks_v3_adds(IksVec3 a, double s) { IksVec3 r; r.x = a.x + s; r.y = a.y + s; r.z = a.z + s; return r; }
static IksVec3 iks_v3_subs(IksVec3 a, double s) { IksVec3 r; r.x = a.x - s; r.y = a.y - s; r.z = a.z - s; return r; }
static IksVec3 iks_v3_divs(IksVec3 a, double s) { IksVec3 r; r.x = a.x / s; r.y = a.y / s; r.z = a.z / s; return r; }
static double iks_v3_dot(IksVec3 a, IksVec3 b) { return a.x * b.x + a.y * b.y + a.z * b.z; }
static double iks_v3_len(IksVec3 a) { return sqrt(a.x * a.x + a.y * a.y + a.z * a.z); }
static IksVec3 iks_v3_norm(IksVec3 a) { IksVec3 r; double l = iks_v3_len(a); r.x = l > 0.0 ? a.x / l : 0.0; r.y = l > 0.0 ? a.y / l : 0.0; r.z = l > 0.0 ? a.z / l : 0.0; return r; }
static IksVec3 iks_v3_lerp(IksVec3 a, IksVec3 b, double t) { IksVec3 r; r.x = a.x + (b.x - a.x) * t; r.y = a.y + (b.y - a.y) * t; r.z = a.z + (b.z - a.z) * t; return r; }
static IksVec3 iks_v3_cross(IksVec3 a, IksVec3 b) { IksVec3 r; r.x = a.y * b.z - a.z * b.y; r.y = a.z * b.x - a.x * b.z; r.z = a.x * b.y - a.y * b.x; return r; }
static char* iks_v3_str(IksVec3 a) { char* s = (char*)malloc(128); if (!s) iks_fail("out of memory"); snprintf(s, 128, "vec3(%.15g, %.15g, %.15g)", a.x, a.y, a.z); return s; }

static IksVec4 iks_v4_add(IksVec4 a, IksVec4 b) { IksVec4 r; r.x = a.x + b.x; r.y = a.y + b.y; r.z = a.z + b.z; r.w = a.w + b.w; return r; }
static IksVec4 iks_v4_sub(IksVec4 a, IksVec4 b) { IksVec4 r; r.x = a.x - b.x; r.y = a.y - b.y; r.z = a.z - b.z; r.w = a.w - b.w; return r; }
static IksVec4 iks_v4_mul(IksVec4 a, IksVec4 b) { IksVec4 r; r.x = a.x * b.x; r.y = a.y * b.y; r.z = a.z * b.z; r.w = a.w * b.w; return r; }
static IksVec4 iks_v4_div(IksVec4 a, IksVec4 b) { IksVec4 r; r.x = a.x / b.x; r.y = a.y / b.y; r.z = a.z / b.z; r.w = a.w / b.w; return r; }
static IksVec4 iks_v4_scale(IksVec4 a, double s) { IksVec4 r; r.x = a.x * s; r.y = a.y * s; r.z = a.z * s; r.w = a.w * s; return r; }
static IksVec4 iks_v4_adds(IksVec4 a, double s) { IksVec4 r; r.x = a.x + s; r.y = a.y + s; r.z = a.z + s; r.w = a.w + s; return r; }
static IksVec4 iks_v4_subs(IksVec4 a, double s) { IksVec4 r; r.x = a.x - s; r.y = a.y - s; r.z = a.z - s; r.w = a.w - s; return r; }
static IksVec4 iks_v4_divs(IksVec4 a, double s) { IksVec4 r; r.x = a.x / s; r.y = a.y / s; r.z = a.z / s; r.w = a.w / s; return r; }
static double iks_v4_dot(IksVec4 a, IksVec4 b) { return a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w; }
static double iks_v4_len(IksVec4 a) { return sqrt(a.x * a.x + a.y * a.y + a.z * a.z + a.w * a.w); }
static IksVec4 iks_v4_norm(IksVec4 a) { IksVec4 r; double l = iks_v4_len(a); r.x = l > 0.0 ? a.x / l : 0.0; r.y = l > 0.0 ? a.y / l : 0.0; r.z = l > 0.0 ? a.z / l : 0.0; r.w = l > 0.0 ? a.w / l : 0.0; return r; }
static IksVec4 iks_v4_lerp(IksVec4 a, IksVec4 b, double t) { IksVec4 r; r.x = a.x + (b.x - a.x) * t; r.y = a.y + (b.y - a.y) * t; r.z = a.z + (b.z - a.z) * t; r.w = a.w + (b.w - a.w) * t; return r; }
static char* iks_v4_str(IksVec4 a) { char* s = (char*)malloc(160); if (!s) iks_fail("out of memory"); snprintf(s, 160, "vec4(%.15g, %.15g, %.15g, %.15g)", a.x, a.y, a.z, a.w); return s; }

static bool iks_v2_eq(IksVec2 a, IksVec2 b) { return a.x == b.x && a.y == b.y; }
static bool iks_v3_eq(IksVec3 a, IksVec3 b) { return a.x == b.x && a.y == b.y && a.z == b.z; }
static bool iks_v4_eq(IksVec4 a, IksVec4 b) { return a.x == b.x && a.y == b.y && a.z == b.z && a.w == b.w; }

static int64_t iks_parse_i(const char* s) {
    return (int64_t)strtoll(s, NULL, 10);
}

static double iks_parse_f(const char* s) {
    return strtod(s, NULL);
}

static char* iks_input(const char* prompt) {
    if (prompt && *prompt) {
        fputs(prompt, stdout);
        fflush(stdout);
    }
    static char line[8192];
    if (!fgets(line, (int)sizeof(line), stdin)) {
        char* s = (char*)malloc(1);
        if (!s) iks_fail("out of memory");
        s[0] = '\0';
        return s;
    }
    size_t n = strlen(line);
    while (n > 0 && (line[n - 1] == '\n' || line[n - 1] == '\r')) line[--n] = '\0';
    char* s = (char*)malloc(n + 1);
    if (!s) iks_fail("out of memory");
    memcpy(s, line, n + 1);
    return s;
}

static void iks_print_arr(IksArray a) {
    putchar('[');
    for (int64_t i = 0; i < a.len; i++) {
        if (i) printf(", ");
        void* p = (char*)a.data + i * a.elemsize;
        switch (a.tag) {
            case IKS_T_INT: printf("%lld", (long long)*(int64_t*)p); break;
            case IKS_T_FLOAT: printf("%.15g", *(double*)p); break;
            case IKS_T_BOOL: printf(*(bool*)p ? "true" : "false"); break;
            case IKS_T_STR: printf("%s", *(char**)p); break;
            default: iks_print_arr(*(IksArray*)p); break;
        }
    }
    putchar(']');
}

static IksArray iks_push(IksArray a, void* value) {
    IksArray b;
    b.len = a.len + 1;
    b.elemsize = a.elemsize;
    b.tag = a.tag;
    b.data = malloc((size_t)(a.elemsize * b.len));
    if (!b.data) iks_fail("out of memory");
    if (a.len > 0) memcpy(b.data, a.data, (size_t)(a.elemsize * a.len));
    memcpy((char*)b.data + a.elemsize * a.len, value, (size_t)a.elemsize);
    return b;
}

static int64_t iks_imin(int64_t a, int64_t b) { return a < b ? a : b; }
static int64_t iks_imax(int64_t a, int64_t b) { return a > b ? a : b; }
static int64_t iks_iclamp(int64_t v, int64_t lo, int64_t hi) {
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

static int iks_rand_seeded = 0;

static double iks_rand(void) {
    if (!iks_rand_seeded) {
        srand((unsigned)time(NULL));
        iks_rand_seeded = 1;
    }
    return (double)rand() / (double)RAND_MAX;
}

static int64_t iks_rand_range(int64_t lo, int64_t hi) {
    if (lo > hi) { int64_t t = lo; lo = hi; hi = t; }
    if (!iks_rand_seeded) {
        srand((unsigned)time(NULL));
        iks_rand_seeded = 1;
    }
    return lo + (int64_t)((double)rand() / ((double)RAND_MAX + 1.0)) * (hi - lo + 1);
}

static char* iks_substr(const char* s, int64_t start, int64_t end) {
    int64_t len = (int64_t)strlen(s);
    if (start < 0) start = 0;
    if (end > len) end = len;
    if (start > end) start = end;
    int64_t n = end - start;
    char* r = (char*)malloc((size_t)n + 1);
    if (!r) iks_fail("out of memory");
    memcpy(r, s + start, (size_t)n);
    r[n] = '\0';
    return r;
}

static char* iks_char_at(const char* s, int64_t i) {
    int64_t len = (int64_t)strlen(s);
    if (i < 0 || i >= len) iks_fail("string index out of bounds");
    char* r = (char*)malloc(2);
    if (!r) iks_fail("out of memory");
    r[0] = s[i];
    r[1] = '\0';
    return r;
}

static int64_t iks_str_find(const char* s, const char* needle) {
    const char* p = strstr(s, needle);
    return p ? (int64_t)(p - s) : -1;
}

static char* iks_str_replace(const char* s, const char* from, const char* to) {
    size_t flen = strlen(from);
    if (flen == 0) {
        char* r = (char*)malloc(strlen(s) + 1);
        if (!r) iks_fail("out of memory");
        strcpy(r, s);
        return r;
    }
    size_t slen = strlen(s);
    size_t tlen = strlen(to);
    size_t count = 0;
    const char* p = s;
    while ((p = strstr(p, from)) != NULL) { count++; p += flen; }
    size_t extra = tlen > flen ? (tlen - flen) * count : 0;
    char* r = (char*)malloc(slen + extra + 1);
    if (!r) iks_fail("out of memory");
    char* w = r;
    const char* cur = s;
    while (1) {
        p = strstr(cur, from);
        if (!p) { strcpy(w, cur); break; }
        size_t n = (size_t)(p - cur);
        memcpy(w, cur, n);
        w += n;
        memcpy(w, to, tlen);
        w += tlen;
        cur = p + flen;
    }
    return r;
}

static char* iks_str_upper(const char* s) {
    int64_t n = (int64_t)strlen(s);
    char* r = (char*)malloc((size_t)n + 1);
    if (!r) iks_fail("out of memory");
    for (int64_t i = 0; i < n; i++) {
        char c = s[i];
        if (c >= 'a' && c <= 'z') c = (char)(c - 32);
        r[i] = c;
    }
    r[n] = '\0';
    return r;
}

static char* iks_str_lower(const char* s) {
    int64_t n = (int64_t)strlen(s);
    char* r = (char*)malloc((size_t)n + 1);
    if (!r) iks_fail("out of memory");
    for (int64_t i = 0; i < n; i++) {
        char c = s[i];
        if (c >= 'A' && c <= 'Z') c = (char)(c + 32);
        r[i] = c;
    }
    r[n] = '\0';
    return r;
}

static char* iks_str_trim(const char* s) {
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') s++;
    int64_t n = (int64_t)strlen(s);
    while (n > 0 && (s[n-1] == ' ' || s[n-1] == '\t' || s[n-1] == '\n' || s[n-1] == '\r')) n--;
    char* r = (char*)malloc((size_t)n + 1);
    if (!r) iks_fail("out of memory");
    memcpy(r, s, (size_t)n);
    r[n] = '\0';
    return r;
}

static int64_t iks_str_starts(const char* s, const char* p) {
    return strncmp(s, p, strlen(p)) == 0 ? 1 : 0;
}

static int64_t iks_str_ends(const char* s, const char* p) {
    size_t sl = strlen(s), pl = strlen(p);
    if (pl > sl) return 0;
    return strcmp(s + sl - pl, p) == 0 ? 1 : 0;
}

static int64_t iks_str_cmp(const char* a, const char* b) {
    return (int64_t)strcmp(a, b);
}

static IksArray iks_str_split(const char* s, const char* sep) {
    IksArray result;
    result.len = 0;
    result.elemsize = (int64_t)sizeof(char*);
    result.tag = IKS_T_STR;
    result.data = NULL;
    size_t seplen = strlen(sep);
    if (seplen == 0) {
        while (*s) {
            char* piece = (char*)malloc(2);
            piece[0] = *s;
            piece[1] = '\0';
            result = iks_push(result, &piece);
            s++;
        }
        return result;
    }
    const char* start = s;
    const char* p;
    while ((p = strstr(start, sep)) != NULL) {
        int64_t n = (int64_t)(p - start);
        char* piece = (char*)malloc((size_t)n + 1);
        memcpy(piece, start, (size_t)n);
        piece[n] = '\0';
        result = iks_push(result, &piece);
        start = p + seplen;
    }
    char* piece = (char*)malloc(strlen(start) + 1);
    strcpy(piece, start);
    result = iks_push(result, &piece);
    return result;
}

static char* iks_str_join(IksArray arr, const char* sep) {
    if (arr.tag != IKS_T_STR) iks_fail("str_join expects a string array");
    size_t seplen = strlen(sep);
    size_t total = 1;
    for (int64_t i = 0; i < arr.len; i++) {
        char* p = *(char**)((char*)arr.data + i * arr.elemsize);
        total += strlen(p);
        if (i > 0) total += seplen;
    }
    char* r = (char*)malloc(total);
    if (!r) iks_fail("out of memory");
    char* w = r;
    for (int64_t i = 0; i < arr.len; i++) {
        char* p = *(char**)((char*)arr.data + i * arr.elemsize);
        if (i > 0) { memcpy(w, sep, seplen); w += seplen; }
        size_t l = strlen(p);
        memcpy(w, p, l);
        w += l;
    }
    *w = '\0';
    return r;
}

static IksArray iks_pop(IksArray a) {
    if (a.len == 0) iks_fail("cannot pop from empty array");
    IksArray b;
    b.len = a.len - 1;
    b.elemsize = a.elemsize;
    b.tag = a.tag;
    b.data = NULL;
    if (b.len > 0) {
        b.data = malloc((size_t)(a.elemsize * b.len));
        if (!b.data) iks_fail("out of memory");
        memcpy(b.data, a.data, (size_t)(a.elemsize * b.len));
    }
    return b;
}

static IksArray iks_arr_remove(IksArray a, int64_t i) {
    if (i < 0 || i >= a.len) iks_fail("array index out of bounds");
    IksArray b;
    b.len = a.len - 1;
    b.elemsize = a.elemsize;
    b.tag = a.tag;
    b.data = NULL;
    if (b.len > 0) {
        b.data = malloc((size_t)(a.elemsize * b.len));
        if (!b.data) iks_fail("out of memory");
        char* d = (char*)b.data;
        char* s = (char*)a.data;
        size_t before = (size_t)(a.elemsize * i);
        size_t after = (size_t)(a.elemsize * (a.len - i - 1));
        memcpy(d, s, before);
        memcpy(d + before, s + before + a.elemsize, after);
    }
    return b;
}

static IksArray iks_arr_insert(IksArray a, int64_t i, void* val) {
    if (i < 0 || i > a.len) iks_fail("array insert index out of bounds");
    IksArray b;
    b.len = a.len + 1;
    b.elemsize = a.elemsize;
    b.tag = a.tag;
    b.data = malloc((size_t)(a.elemsize * b.len));
    if (!b.data) iks_fail("out of memory");
    char* d = (char*)b.data;
    char* s = (char*)a.data;
    size_t before = (size_t)(a.elemsize * i);
    size_t after = (size_t)(a.elemsize * (a.len - i));
    if (before > 0) memcpy(d, s, before);
    memcpy(d + before, val, (size_t)a.elemsize);
    if (after > 0) memcpy(d + before + a.elemsize, s + before, after);
    return b;
}

static IksArray iks_arr_slice(IksArray a, int64_t start, int64_t end) {
    int64_t len = a.len;
    if (start < 0) start = 0;
    if (end > len) end = len;
    if (start > end) start = end;
    int64_t n = end - start;
    IksArray b;
    b.len = n;
    b.elemsize = a.elemsize;
    b.tag = a.tag;
    b.data = NULL;
    if (n > 0) {
        b.data = malloc((size_t)(a.elemsize * n));
        if (!b.data) iks_fail("out of memory");
        memcpy(b.data, (char*)a.data + a.elemsize * start, (size_t)(a.elemsize * n));
    }
    return b;
}

static IksArray iks_arr_reverse(IksArray a) {
    IksArray b;
    b.len = a.len;
    b.elemsize = a.elemsize;
    b.tag = a.tag;
    b.data = NULL;
    if (a.len > 0) {
        b.data = malloc((size_t)(a.elemsize * a.len));
        if (!b.data) iks_fail("out of memory");
        for (int64_t i = 0; i < a.len; i++) {
            memcpy((char*)b.data + (a.len - 1 - i) * a.elemsize,
                   (char*)a.data + i * a.elemsize, (size_t)a.elemsize);
        }
    }
    return b;
}

static int iks_cmp_i64(const void* a, const void* b) {
    int64_t x = *(const int64_t*)a, y = *(const int64_t*)b;
    return (x > y) - (x < y);
}

static int iks_cmp_f64(const void* a, const void* b) {
    double x = *(const double*)a, y = *(const double*)b;
    return (x > y) - (x < y);
}

static IksArray iks_arr_sort(IksArray a) {
    IksArray b;
    b.len = a.len;
    b.elemsize = a.elemsize;
    b.tag = a.tag;
    b.data = NULL;
    if (a.len > 0) {
        b.data = malloc((size_t)(a.elemsize * a.len));
        if (!b.data) iks_fail("out of memory");
        memcpy(b.data, a.data, (size_t)(a.elemsize * a.len));
        if (a.tag == IKS_T_INT)
            qsort(b.data, (size_t)b.len, (size_t)a.elemsize, iks_cmp_i64);
        else if (a.tag == IKS_T_FLOAT)
            qsort(b.data, (size_t)b.len, (size_t)a.elemsize, iks_cmp_f64);
        else
            iks_fail("sort only works on int[] and float[]");
    }
    return b;
}

static int64_t iks_arr_index_of(IksArray a, void* val) {
    for (int64_t i = 0; i < a.len; i++) {
        void* p = (char*)a.data + i * a.elemsize;
        int found = 0;
        switch (a.tag) {
            case IKS_T_INT: found = *(int64_t*)p == *(int64_t*)val; break;
            case IKS_T_FLOAT: found = *(double*)p == *(double*)val; break;
            case IKS_T_BOOL: found = *(bool*)p == *(bool*)val; break;
            case IKS_T_STR: found = strcmp(*(char**)p, *(char**)val) == 0; break;
        }
        if (found) return i;
    }
    return -1;
}

static int64_t iks_arr_contains(IksArray a, void* val) {
    return iks_arr_index_of(a, val) >= 0 ? 1 : 0;
}

static char* iks_file_read(const char* path) {
    FILE* f = fopen(path, "rb");
    if (!f) iks_fail("cannot open file for reading");
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (n < 0) n = 0;
    char* s = (char*)malloc((size_t)n + 1);
    if (!s) iks_fail("out of memory");
    size_t got = fread(s, 1, (size_t)n, f);
    s[got] = '\0';
    fclose(f);
    return s;
}

static int64_t iks_file_write(const char* path, const char* content) {
    FILE* f = fopen(path, "wb");
    if (!f) return 0;
    fputs(content, f);
    fclose(f);
    return 1;
}

static int64_t iks_file_exists(const char* path) {
    FILE* f = fopen(path, "rb");
    if (f) { fclose(f); return 1; }
    return 0;
}

static int64_t iks_file_delete(const char* path) {
    return remove(path) == 0 ? 1 : 0;
}

typedef struct { char* key; int tag; union { int64_t i; double f; bool b; char* s; } val; } IksMapEntry;
typedef struct { int64_t len; int64_t cap; IksMapEntry* entries; } IksMap;

static char* iks_strdup(const char* s) {
    size_t n = strlen(s);
    char* r = (char*)malloc(n + 1);
    if (!r) iks_fail("out of memory");
    memcpy(r, s, n + 1);
    return r;
}

static IksMap* iks_map_new(void) {
    IksMap* m = (IksMap*)malloc(sizeof(IksMap));
    if (!m) iks_fail("out of memory");
    m->len = 0;
    m->cap = 0;
    m->entries = NULL;
    return m;
}

static int64_t iks_map_find(IksMap* m, const char* key) {
    for (int64_t i = 0; i < m->len; i++) {
        if (strcmp(m->entries[i].key, key) == 0) return i;
    }
    return -1;
}

static IksMapEntry* iks_map_slot(IksMap* m, const char* key) {
    int64_t idx = iks_map_find(m, key);
    if (idx >= 0) return &m->entries[idx];
    if (m->len >= m->cap) {
        int64_t nc = m->cap ? m->cap * 2 : 16;
        IksMapEntry* ne = (IksMapEntry*)realloc(m->entries, (size_t)nc * sizeof(IksMapEntry));
        if (!ne) iks_fail("out of memory");
        m->entries = ne;
        m->cap = nc;
    }
    IksMapEntry* e = &m->entries[m->len++];
    e->key = iks_strdup(key);
    e->tag = -1;
    return e;
}

static void iks_map_set_i(IksMap* m, const char* key, int64_t v) {
    IksMapEntry* e = iks_map_slot(m, key);
    e->tag = IKS_T_INT;
    e->val.i = v;
}
static void iks_map_set_f(IksMap* m, const char* key, double v) {
    IksMapEntry* e = iks_map_slot(m, key);
    e->tag = IKS_T_FLOAT;
    e->val.f = v;
}
static void iks_map_set_b(IksMap* m, const char* key, bool v) {
    IksMapEntry* e = iks_map_slot(m, key);
    e->tag = IKS_T_BOOL;
    e->val.b = v;
}
static void iks_map_set_s(IksMap* m, const char* key, const char* v) {
    IksMapEntry* e = iks_map_slot(m, key);
    e->tag = IKS_T_STR;
    e->val.s = iks_strdup(v);
}

static int64_t iks_map_get_i(IksMap* m, const char* key, int64_t def) {
    int64_t idx = iks_map_find(m, key);
    if (idx < 0) return def;
    IksMapEntry* e = &m->entries[idx];
    if (e->tag == IKS_T_INT) return e->val.i;
    if (e->tag == IKS_T_FLOAT) return (int64_t)e->val.f;
    iks_fail("map get type mismatch: expected int");
    return def;
}
static double iks_map_get_f(IksMap* m, const char* key, double def) {
    int64_t idx = iks_map_find(m, key);
    if (idx < 0) return def;
    IksMapEntry* e = &m->entries[idx];
    if (e->tag == IKS_T_FLOAT) return e->val.f;
    if (e->tag == IKS_T_INT) return (double)e->val.i;
    iks_fail("map get type mismatch: expected float");
    return def;
}
static bool iks_map_get_b(IksMap* m, const char* key, bool def) {
    int64_t idx = iks_map_find(m, key);
    if (idx < 0) return def;
    IksMapEntry* e = &m->entries[idx];
    if (e->tag == IKS_T_BOOL) return e->val.b;
    iks_fail("map get type mismatch: expected bool");
    return def;
}
static char* iks_map_get_s(IksMap* m, const char* key, const char* def) {
    int64_t idx = iks_map_find(m, key);
    if (idx < 0) return (char*)def;
    IksMapEntry* e = &m->entries[idx];
    if (e->tag == IKS_T_STR) return e->val.s;
    iks_fail("map get type mismatch: expected string");
    return (char*)def;
}

static int64_t iks_map_has(IksMap* m, const char* key) {
    return iks_map_find(m, key) >= 0 ? 1 : 0;
}

static int64_t iks_map_remove(IksMap* m, const char* key) {
    int64_t idx = iks_map_find(m, key);
    if (idx < 0) return 0;
    free(m->entries[idx].key);
    if (m->entries[idx].tag == IKS_T_STR) free(m->entries[idx].val.s);
    m->entries[idx] = m->entries[m->len - 1];
    m->len--;
    return 1;
}

static int64_t iks_map_len(IksMap* m) {
    return m->len;
}

static IksArray iks_map_keys(IksMap* m) {
    IksArray result;
    result.len = 0;
    result.elemsize = (int64_t)sizeof(char*);
    result.tag = IKS_T_STR;
    result.data = NULL;
    for (int64_t i = 0; i < m->len; i++) {
        char* k = iks_strdup(m->entries[i].key);
        result = iks_push(result, &k);
    }
    return result;
}

#if defined(IKS_WIN)

static HWND iks_hwnd;
static BITMAPINFO iks_bmi;
static void* iks_pixels;
static int iks_gw = 0, iks_gh = 0;
static int iks_gfx_on = 0, iks_gfx_closed = 0;
static double iks_gfx_t0 = 0.0;
static unsigned char iks_keys[256];
static int iks_mx = 0, iks_my = 0;
static unsigned char iks_mbtn[3];
static HGLRC iks_glrc;
static HDC iks_gldc;
static ID3D11Device* iks_ddev;
static ID3D11DeviceContext* iks_dctx;
static IDXGISwapChain* iks_dsc;
static ID3D11RenderTargetView* iks_drv;
static ID3D11VertexShader* iks_dvs;
static ID3D11PixelShader* iks_dps;
static ID3D11InputLayout* iks_dil;
static int iks_vkon = 0;

static double iks_now(void) {
    static LARGE_INTEGER freq, t;
    static int init = 0;
    if (!init) {
        QueryPerformanceFrequency(&freq);
        init = 1;
    }
    QueryPerformanceCounter(&t);
    return (double)t.QuadPart / (double)freq.QuadPart;
}

static LRESULT CALLBACK iks_wndproc(HWND h, UINT m, WPARAM w, LPARAM l) {
    switch (m) {
        case WM_DESTROY:
            iks_gfx_closed = 1;
            PostQuitMessage(0);
            return 0;
        case WM_PAINT: {
            PAINTSTRUCT ps;
            HDC dc = BeginPaint(h, &ps);
            if (iks_pixels && !iks_glrc && !iks_ddev && !iks_vkon) {
                StretchDIBits(dc, 0, 0, iks_gw, iks_gh, 0, 0, iks_gw, iks_gh,
                              iks_pixels, &iks_bmi, DIB_RGB_COLORS, SRCCOPY);
            }
            EndPaint(h, &ps);
            return 0;
        }
        case WM_KEYDOWN:
            if (w < 256) iks_keys[w] = 1;
            return 0;
        case WM_KEYUP:
            if (w < 256) iks_keys[w] = 0;
            return 0;
        case WM_MOUSEMOVE:
            iks_mx = (int)(short)LOWORD(l);
            iks_my = (int)(short)HIWORD(l);
            return 0;
        case WM_LBUTTONDOWN: iks_mbtn[0] = 1; return 0;
        case WM_LBUTTONUP: iks_mbtn[0] = 0; return 0;
        case WM_RBUTTONDOWN: iks_mbtn[1] = 1; return 0;
        case WM_RBUTTONUP: iks_mbtn[1] = 0; return 0;
        case WM_MBUTTONDOWN: iks_mbtn[2] = 1; return 0;
        case WM_MBUTTONUP: iks_mbtn[2] = 0; return 0;
    }
    return DefWindowProcA(h, m, w, l);
}

static void iks_gfx_open(int64_t w, int64_t h, const char* title) {
    if (iks_gfx_on) return;
    if (w < 64 || h < 64 || w > 3840 || h > 2160) {
        fprintf(stderr, "gfx error: window size must be between 64x64 and 3840x2160\n");
        exit(1);
    }
    iks_gw = (int)w;
    iks_gh = (int)h;
    WNDCLASSA wc;
    ZeroMemory(&wc, sizeof(wc));
    wc.style = CS_HREDRAW | CS_VREDRAW | CS_OWNDC;
    wc.lpfnWndProc = iks_wndproc;
    wc.hInstance = GetModuleHandleA(NULL);
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    wc.lpszClassName = "IksGfxWindow";
    RegisterClassA(&wc);
    RECT r;
    r.left = 0;
    r.top = 0;
    r.right = iks_gw;
    r.bottom = iks_gh;
    DWORD style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    AdjustWindowRect(&r, style, FALSE);
    iks_hwnd = CreateWindowExA(0, "IksGfxWindow", title ? title : "IKS", style,
        CW_USEDEFAULT, CW_USEDEFAULT, r.right - r.left, r.bottom - r.top,
        NULL, NULL, wc.hInstance, NULL);
    if (!iks_hwnd) {
        fprintf(stderr, "gfx error: cannot create window\n");
        exit(1);
    }
    ZeroMemory(&iks_bmi, sizeof(iks_bmi));
    iks_bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    iks_bmi.bmiHeader.biWidth = iks_gw;
    iks_bmi.bmiHeader.biHeight = -iks_gh;
    iks_bmi.bmiHeader.biPlanes = 1;
    iks_bmi.bmiHeader.biBitCount = 32;
    iks_bmi.bmiHeader.biCompression = BI_RGB;
    iks_pixels = calloc((size_t)iks_gw * (size_t)iks_gh, 4);
    if (!iks_pixels) {
        fprintf(stderr, "gfx error: out of memory\n");
        exit(1);
    }
    iks_gfx_t0 = iks_now();
    ShowWindow(iks_hwnd, SW_SHOW);
    UpdateWindow(iks_hwnd);
    iks_gfx_on = 1;
}

static int64_t iks_gfx_poll(void) {
    if (!iks_gfx_on || iks_gfx_closed) return 0;
    MSG msg;
    while (PeekMessageA(&msg, NULL, 0, 0, PM_REMOVE)) {
        if (msg.message == WM_QUIT) {
            iks_gfx_closed = 1;
        } else {
            TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }
    }
    return iks_gfx_closed ? 0 : 1;
}

static void iks_gfx_present(void) {
    if (!iks_gfx_on || !iks_hwnd) return;
    HDC dc = GetDC(iks_hwnd);
    StretchDIBits(dc, 0, 0, iks_gw, iks_gh, 0, 0, iks_gw, iks_gh,
                  iks_pixels, &iks_bmi, DIB_RGB_COLORS, SRCCOPY);
    ReleaseDC(iks_hwnd, dc);
}

static unsigned char iks_clamp8(int64_t v) {
    if (v < 0) return 0;
    if (v > 255) return 255;
    return (unsigned char)v;
}

static void iks_gfx_clear(int64_t r, int64_t g, int64_t b) {
    if (!iks_pixels) return;
    unsigned char cr = iks_clamp8(r), cg = iks_clamp8(g), cb = iks_clamp8(b);
    unsigned int px = (unsigned int)cb | ((unsigned int)cg << 8) | ((unsigned int)cr << 16);
    unsigned int* p = (unsigned int*)iks_pixels;
    size_t n = (size_t)iks_gw * (size_t)iks_gh;
    for (size_t i = 0; i < n; i++) p[i] = px;
}

static void iks_gfx_pixel(int64_t x, int64_t y, int64_t r, int64_t g, int64_t b) {
    if (!iks_pixels) return;
    if (x < 0 || y < 0 || x >= iks_gw || y >= iks_gh) return;
    unsigned char* p = (unsigned char*)iks_pixels + (y * iks_gw + x) * 4;
    p[0] = iks_clamp8(b);
    p[1] = iks_clamp8(g);
    p[2] = iks_clamp8(r);
}

static void iks_gfx_add(int64_t x, int64_t y, int64_t r, int64_t g, int64_t b) {
    if (!iks_pixels) return;
    if (x < 0 || y < 0 || x >= iks_gw || y >= iks_gh) return;
    unsigned char* p = (unsigned char*)iks_pixels + (y * iks_gw + x) * 4;
    int rr = (int)p[2] + (int)r;
    int gg = (int)p[1] + (int)g;
    int bb = (int)p[0] + (int)b;
    p[2] = (unsigned char)(rr > 255 ? 255 : rr);
    p[1] = (unsigned char)(gg > 255 ? 255 : gg);
    p[0] = (unsigned char)(bb > 255 ? 255 : bb);
}

static void iks_gfx_line(int64_t x0, int64_t y0, int64_t x1, int64_t y1,
                         int64_t r, int64_t g, int64_t b) {
    int64_t dx = x1 > x0 ? x1 - x0 : x0 - x1;
    int64_t dy = y1 > y0 ? y1 - y0 : y0 - y1;
    int64_t sx = x0 < x1 ? 1 : -1;
    int64_t sy = y0 < y1 ? 1 : -1;
    int64_t err = dx - dy;
    for (;;) {
        iks_gfx_pixel(x0, y0, r, g, b);
        if (x0 == x1 && y0 == y1) break;
        int64_t e2 = 2 * err;
        if (e2 > -dy) {
            err -= dy;
            x0 += sx;
        }
        if (e2 < dx) {
            err += dx;
            y0 += sy;
        }
    }
}

static int64_t iks_gfx_width(void) { return iks_gw; }
static int64_t iks_gfx_height(void) { return iks_gh; }
static double iks_gfx_time(void) { return iks_now() - iks_gfx_t0; }
static int64_t iks_gfx_mouse_x(void) { return iks_mx; }
static int64_t iks_gfx_mouse_y(void) { return iks_my; }

static int64_t iks_gfx_key_down(int64_t vk) {
    if (vk < 0 || vk >= 256) return 0;
    return iks_keys[vk] ? 1 : 0;
}

static int64_t iks_gfx_mouse_button(int64_t b) {
    if (b < 0 || b >= 3) return 0;
    return iks_mbtn[b] ? 1 : 0;
}

static void iks_sleep(int64_t ms) { Sleep((DWORD)ms); }

typedef void (APIENTRY *IKS_PFNGLGENQUERIES)(GLsizei, GLuint*);
typedef void (APIENTRY *IKS_PFNGLBEGINQUERY)(GLenum, GLuint);
typedef void (APIENTRY *IKS_PFNGLENDQUERY)(GLenum);
typedef void (APIENTRY *IKS_PFNGLGETQUERYOBJECTIV)(GLuint, GLenum, GLint*);
typedef void (APIENTRY *IKS_PFNGLGETQUERYOBJECTUI64V)(GLuint, GLenum, unsigned long long*);
static IKS_PFNGLGENQUERIES iks_pGenQueries;
static IKS_PFNGLBEGINQUERY iks_pBeginQuery;
static IKS_PFNGLENDQUERY iks_pEndQuery;
static IKS_PFNGLGETQUERYOBJECTIV iks_pGetQueryObjectiv;
static IKS_PFNGLGETQUERYOBJECTUI64V iks_pGetQueryObjectui64v;
static GLuint iks_gpu_query[4] = {0, 0, 0, 0};
static int iks_gpu_ok = 0;
static int iks_gpu_open = 0;
static int iks_gpu_idx = 0;
static unsigned iks_gpu_calls = 0;
static double iks_gpu_t0s[4] = {0.0, 0.0, 0.0, 0.0};
static double iks_gpu_end_t[4] = {0.0, 0.0, 0.0, 0.0};
static double iks_gpu_pct = 0.0;

static PDH_HQUERY iks_pdh_q = NULL;
static HCOUNTER iks_pdh_c = NULL;
static int iks_pdh_ok = 0;
static int iks_pdh_tried = 0;
static double iks_pdh_pct = 0.0;

static void iks_pdh_init(void) {
    if (iks_pdh_tried) return;
    iks_pdh_tried = 1;
    if (PdhOpenQueryA(NULL, 0, &iks_pdh_q) != ERROR_SUCCESS) { iks_pdh_q = NULL; return; }
    if (PdhAddEnglishCounterA(iks_pdh_q, "\\GPU Engine(*)\\Utilization Percentage", 0, &iks_pdh_c)
        != ERROR_SUCCESS) {
        PdhCloseQuery(iks_pdh_q);
        iks_pdh_q = NULL;
        return;
    }
    PdhCollectQueryData(iks_pdh_q);
    iks_pdh_ok = 1;
}

static void iks_gl_swap_query_end(void) {
    if (!iks_gpu_ok) return;
    if (iks_gpu_open) {
        iks_pEndQuery(0x88BF);
        iks_gpu_end_t[iks_gpu_idx] = iks_now();
        iks_gpu_open = 0;
    }
    unsigned c = iks_gpu_calls;
    if (c >= 4) {
        int rdi = (int)((c - 3) % 4);
        GLint avail = 0;
        iks_pGetQueryObjectiv(iks_gpu_query[rdi], 0x8867, &avail);
        if (avail) {
            unsigned long long ns = 0;
            iks_pGetQueryObjectui64v(iks_gpu_query[rdi], 0x8866, &ns);
            double wall = iks_gpu_end_t[rdi] - iks_gpu_t0s[rdi];
            if (wall > 0.0) {
                double pct = (double)ns * 1e-9 / wall * 100.0;
                if (pct <= 150.0) iks_gpu_pct = pct;
            }
        }
    }
    int ni = (int)(c % 4);
    iks_pBeginQuery(0x88BF, iks_gpu_query[ni]);
    iks_gpu_idx = ni;
    iks_gpu_open = 1;
    iks_gpu_t0s[ni] = iks_now();
    iks_gpu_calls++;
}

static double iks_gpu_load(void) {
    iks_pdh_init();
    if (iks_pdh_ok) {
        if (PdhCollectQueryData(iks_pdh_q) == ERROR_SUCCESS) {
            DWORD buf_size = 0;
            DWORD item_count = 0;
            PDH_FMT_COUNTERVALUE_ITEM_A *items = NULL;
            PdhGetFormattedCounterArrayA(iks_pdh_c, PDH_FMT_DOUBLE, &buf_size, &item_count, NULL);
            if (buf_size > 0) {
                items = (PDH_FMT_COUNTERVALUE_ITEM_A *)malloc(buf_size);
                if (items
                    && PdhGetFormattedCounterArrayA(iks_pdh_c, PDH_FMT_DOUBLE, &buf_size,
                                                    &item_count, items) == ERROR_SUCCESS) {
                    double s = 0.0;
                    DWORD i;
                    for (i = 0; i < item_count; i++) s += items[i].FmtValue.doubleValue;
                    if (s > 100.0) s = 100.0;
                    if (s < 0.0) s = 0.0;
                    iks_pdh_pct = s;
                }
                free(items);
            }
        }
        return iks_pdh_pct;
    }
    return iks_gpu_pct;
}

static double iks_cpu_load(void) {
    static ULONGLONG last_wall = 0;
    static ULONGLONG last_cpu = 0;
    FILETIME cr, ex, kr, ut;
    SYSTEM_INFO si;
    if (!GetProcessTimes(GetCurrentProcess(), &cr, &ex, &kr, &ut)) return 0.0;
    ULARGE_INTEGER k, u;
    k.LowPart = kr.dwLowDateTime;
    k.HighPart = kr.dwHighDateTime;
    u.LowPart = ut.dwLowDateTime;
    u.HighPart = ut.dwHighDateTime;
    ULONGLONG cpu = k.QuadPart + u.QuadPart;
    ULONGLONG wall = GetTickCount64();
    if (!last_wall) {
        last_wall = wall;
        last_cpu = cpu;
        return 0.0;
    }
    double dcpu = (double)(cpu - last_cpu) / 10000000.0;
    double dwall = (double)(wall - last_wall) / 1000.0;
    last_wall = wall;
    last_cpu = cpu;
    if (dwall <= 0.0) return 0.0;
    GetSystemInfo(&si);
    DWORD ncpu = si.dwNumberOfProcessors;
    if (ncpu < 1) ncpu = 1;
    double pct = 100.0 * dcpu / (dwall * (double)ncpu);
    if (pct > 100.0) pct = 100.0;
    return pct;
}

/* ---- NPU detection: PnP description heuristic (no device-ID guessing) ---- */
static int iks_npu_n = -1;
static char iks_npu_nm[192] = "";

static int iks_npu_word(const char* hay, const char* w) {
    size_t wl = strlen(w);
    const char* p = hay;
    while ((p = strstr(p, w)) != NULL) {
        char bc = (p == hay) ? 0 : p[-1];
        char ac = p[wl];
        int bok = !((bc >= 'a' && bc <= 'z') || (bc >= '0' && bc <= '9'));
        int aok = !((ac >= 'a' && ac <= 'z') || (ac >= '0' && ac <= '9'));
        if (bok && aok) return 1;
        p++;
    }
    return 0;
}

static int iks_npu_look(const char* s) {
    char b[512];
    int i, n = 0;
    if (!s) return 0;
    for (i = 0; s[i] && n < 511; i++) {
        char c = s[i];
        if (c >= 'A' && c <= 'Z') c = (char)(c - 'A' + 'a');
        b[n++] = c;
    }
    b[n] = 0;
    if (iks_npu_word(b, "npu")) return 1;
    if (strstr(b, "neural")) return 1;
    if (strstr(b, "xdna")) return 1;
    if (strstr(b, "hexagon")) return 1;
    if (strstr(b, "myriad")) return 1;
    if (strstr(b, "ai boost")) return 1;
    if (strstr(b, "ai accelerator")) return 1;
    if (strstr(b, "ai acceleration")) return 1;
    if (strstr(b, "ai engine")) return 1;
    return 0;
}

static void iks_npu_take(const char* s) {
    int n = 0;
    if (!s) s = "";
    while (*s && *s != '\n' && *s != '\r' && n < (int)sizeof(iks_npu_nm) - 1)
        iks_npu_nm[n++] = *s++;
    iks_npu_nm[n] = 0;
}

static void iks_npu_scan(void) {
    HKEY root = NULL, bus = NULL, dev = NULL, inst = NULL;
    DWORD ni = 0;
    if (iks_npu_n >= 0) return;
    iks_npu_n = 0;
    iks_npu_nm[0] = 0;
    if (RegOpenKeyExA(HKEY_LOCAL_MACHINE, "SYSTEM\\CurrentControlSet\\Enum",
                      0, KEY_READ, &root) != ERROR_SUCCESS) return;
    for (;;) {
        char bn[320]; DWORD bl = (DWORD)sizeof(bn);
        if (RegEnumKeyExA(root, ni, bn, &bl, NULL, NULL, NULL, NULL) != ERROR_SUCCESS) break;
        ni++;
        if (RegOpenKeyExA(root, bn, 0, KEY_READ, &bus) != ERROR_SUCCESS) continue;
        {
            DWORD di = 0;
            for (;;) {
                char dn[320]; DWORD dl = (DWORD)sizeof(dn);
                if (RegEnumKeyExA(bus, di, dn, &dl, NULL, NULL, NULL, NULL) != ERROR_SUCCESS) break;
                di++;
                if (RegOpenKeyExA(bus, dn, 0, KEY_READ, &dev) != ERROR_SUCCESS) continue;
                {
                    DWORD ii = 0;
                    for (;;) {
                        char ik[320]; DWORD il = (DWORD)sizeof(ik);
                        if (RegEnumKeyExA(dev, ii, ik, &il, NULL, NULL, NULL, NULL) != ERROR_SUCCESS) break;
                        ii++;
                        if (RegOpenKeyExA(dev, ik, 0, KEY_READ, &inst) != ERROR_SUCCESS) continue;
                        {
                            char d[512];
                            DWORD n = (DWORD)sizeof(d) - 1;
                            memset(d, 0, sizeof(d));
                            if (RegQueryValueExA(inst, "DeviceDesc", NULL, NULL,
                                                 (LPBYTE)d, &n) == ERROR_SUCCESS) {
                                char* semi;
                                d[sizeof(d) - 1] = 0;
                                semi = strrchr(d, ';');
                                if (semi && semi[1] && iks_npu_look(semi + 1)) {
                                    if (iks_npu_n == 0) iks_npu_take(semi + 1);
                                    iks_npu_n++;
                                }
                            }
                        }
                        RegCloseKey(inst);
                    }
                }
                RegCloseKey(dev);
            }
        }
        RegCloseKey(bus);
    }
    RegCloseKey(root);
}

static int64_t iks_npu_available(void) { iks_npu_scan(); return iks_npu_n > 0 ? 1 : 0; }
static int64_t iks_npu_count(void) { iks_npu_scan(); return (int64_t)iks_npu_n; }
static char* iks_npu_name(void) { iks_npu_scan(); return iks_npu_nm; }
static double iks_npu_tops(void) { return 0.0; }
static double iks_npu_load(void) { return 0.0; }

static void iks_gl_init(int64_t w, int64_t h, const char* title) {
    if (!iks_gfx_on) iks_gfx_open(w, h, title);
    if (iks_glrc) return;
    iks_gldc = GetDC(iks_hwnd);
    PIXELFORMATDESCRIPTOR pfd;
    ZeroMemory(&pfd, sizeof(pfd));
    pfd.nSize = sizeof(pfd);
    pfd.nVersion = 1;
    pfd.dwFlags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER;
    pfd.iPixelType = PFD_TYPE_RGBA;
    pfd.cColorBits = 32;
    pfd.cDepthBits = 24;
    int pf = ChoosePixelFormat(iks_gldc, &pfd);
    if (!pf || !SetPixelFormat(iks_gldc, pf, &pfd)) {
        fprintf(stderr, "gfx error: cannot set pixel format\n");
        exit(1);
    }
    iks_glrc = wglCreateContext(iks_gldc);
    if (!iks_glrc || !wglMakeCurrent(iks_gldc, iks_glrc)) {
        fprintf(stderr, "gfx error: cannot create OpenGL context\n");
        exit(1);
    }
    glEnable(GL_DEPTH_TEST);
    glDepthFunc(GL_LEQUAL);
    glClearColor(0.0f, 0.0f, 0.0f, 1.0f);
    typedef BOOL (WINAPI *iks_wgl_swapinterval_t)(int);
    iks_wgl_swapinterval_t iks_wgl_swapinterval =
        (iks_wgl_swapinterval_t)wglGetProcAddress("wglSwapIntervalEXT");
    if (iks_wgl_swapinterval) iks_wgl_swapinterval(0);
    iks_pGenQueries = (IKS_PFNGLGENQUERIES)wglGetProcAddress("glGenQueries");
    iks_pBeginQuery = (IKS_PFNGLBEGINQUERY)wglGetProcAddress("glBeginQuery");
    iks_pEndQuery = (IKS_PFNGLENDQUERY)wglGetProcAddress("glEndQuery");
    iks_pGetQueryObjectiv = (IKS_PFNGLGETQUERYOBJECTIV)wglGetProcAddress("glGetQueryObjectiv");
    iks_pGetQueryObjectui64v = (IKS_PFNGLGETQUERYOBJECTUI64V)wglGetProcAddress("glGetQueryObjectui64v");
    if (!iks_pGenQueries) iks_pGenQueries = (IKS_PFNGLGENQUERIES)wglGetProcAddress("glGenQueriesARB");
    if (!iks_pBeginQuery) iks_pBeginQuery = (IKS_PFNGLBEGINQUERY)wglGetProcAddress("glBeginQueryARB");
    if (!iks_pEndQuery) iks_pEndQuery = (IKS_PFNGLENDQUERY)wglGetProcAddress("glEndQueryARB");
    if (!iks_pGetQueryObjectiv) iks_pGetQueryObjectiv = (IKS_PFNGLGETQUERYOBJECTIV)wglGetProcAddress("glGetQueryObjectivARB");
    if (!iks_pGetQueryObjectui64v) iks_pGetQueryObjectui64v = (IKS_PFNGLGETQUERYOBJECTUI64V)wglGetProcAddress("glGetQueryObjectui64vARB");
    if (iks_pGenQueries && iks_pBeginQuery && iks_pEndQuery &&
        iks_pGetQueryObjectiv && iks_pGetQueryObjectui64v) {
        iks_pGenQueries(4, iks_gpu_query);
        iks_gpu_ok = iks_gpu_query[0] != 0 && iks_gpu_query[1] != 0 &&
                     iks_gpu_query[2] != 0 && iks_gpu_query[3] != 0;
    }
}

static void iks_gl_clear(int64_t r, int64_t g, int64_t b) {
    glClearColor((float)iks_clamp8(r) / 255.0f,
                 (float)iks_clamp8(g) / 255.0f,
                 (float)iks_clamp8(b) / 255.0f, 1.0f);
    glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
}

static void iks_gl_view(int64_t x, int64_t y, int64_t w, int64_t h) {
    glViewport((GLint)x, (GLint)y, (GLsizei)w, (GLsizei)h);
}

static void iks_gl_proj(double fov, double aspect, double zn, double zf) {
    if (zn <= 0.0 || zf <= zn) {
        fprintf(stderr, "gfx error: bad frustum (need 0 < near < far)\n");
        exit(1);
    }
    double top = zn * tan(fov * 3.14159265358979323846 / 360.0);
    glMatrixMode(GL_PROJECTION);
    glLoadIdentity();
    glFrustum(-top * aspect, top * aspect, -top, top, zn, zf);
    glMatrixMode(GL_MODELVIEW);
}

static void iks_gl_push(void) { glPushMatrix(); }
static void iks_gl_pop(void) { glPopMatrix(); }

static void iks_gl_translate(double x, double y, double z) {
    glTranslated(x, y, z);
}

static void iks_gl_rotate(double a, double x, double y, double z) {
    glRotated(a, x, y, z);
}

static void iks_gl_scale(double x, double y, double z) {
    glScaled(x, y, z);
}

typedef struct { float x, y, z; GLubyte r, g, b, a; } IksV;
static IksV* iks_vbuf = NULL;
static int iks_vn = 0;
static int iks_vcap = 0;
static int iks_draw_mode = 0;
static int iks_in_begin = 0;
static GLubyte iks_cr = 255, iks_cg = 255, iks_cb = 255, iks_ca = 255;

static void iks_gl_color(int64_t r, int64_t g, int64_t b, int64_t a) {
    iks_cr = (GLubyte)iks_clamp8(r);
    iks_cg = (GLubyte)iks_clamp8(g);
    iks_cb = (GLubyte)iks_clamp8(b);
    iks_ca = (GLubyte)iks_clamp8(a);
    glColor4ub(iks_cr, iks_cg, iks_cb, iks_ca);
}

static void iks_gl_begin(int64_t mode) {
    GLenum m;
    switch (mode) {
        case 0: m = GL_POINTS; break;
        case 1: m = GL_LINES; break;
        case 2: m = GL_LINE_STRIP; break;
        case 4: m = GL_TRIANGLE_STRIP; break;
        case 5: m = GL_TRIANGLE_FAN; break;
        case 6: m = GL_QUADS; break;
        case 7: m = GL_QUAD_STRIP; break;
        default: m = GL_TRIANGLES; break;
    }
    iks_draw_mode = (int)m;
    iks_vn = 0;
    iks_in_begin = 1;
}

static void iks_gl_vertex(double x, double y, double z) {
    if (!iks_in_begin) return;
    if (iks_vn >= iks_vcap) {
        int nc = iks_vcap ? iks_vcap * 2 : 4096;
        IksV* nb = (IksV*)realloc(iks_vbuf, (size_t)nc * sizeof(IksV));
        if (!nb) {
            fprintf(stderr, "out of memory\n");
            exit(1);
        }
        iks_vbuf = nb;
        iks_vcap = nc;
    }
    IksV* v = &iks_vbuf[iks_vn++];
    v->x = (float)x;
    v->y = (float)y;
    v->z = (float)z;
    v->r = iks_cr;
    v->g = iks_cg;
    v->b = iks_cb;
    v->a = iks_ca;
}

static void iks_gl_end(void) {
    if (!iks_in_begin) return;
    iks_in_begin = 0;
    if (iks_vn <= 0) return;
    glEnableClientState(GL_VERTEX_ARRAY);
    glEnableClientState(GL_COLOR_ARRAY);
    glVertexPointer(3, GL_FLOAT, sizeof(IksV), &iks_vbuf[0].x);
    glColorPointer(4, GL_UNSIGNED_BYTE, sizeof(IksV), &iks_vbuf[0].r);
    glDrawArrays((GLenum)iks_draw_mode, 0, iks_vn);
    glDisableClientState(GL_COLOR_ARRAY);
    glDisableClientState(GL_VERTEX_ARRAY);
    iks_vn = 0;
}

static void iks_gl_rect(double x, double y, double w, double h) {
    iks_gl_begin(6);
    iks_gl_vertex(x, y, 0); iks_gl_vertex(x + w, y, 0); iks_gl_vertex(x + w, y + h, 0); iks_gl_vertex(x, y + h, 0);
    iks_gl_end();
}

static void iks_gl_circle(double cx, double cy, double r, int64_t segs) {
    if (segs < 3) segs = 32;
    iks_gl_begin(5);
    iks_gl_vertex(cx, cy, 0);
    double step = 6.283185307179586 / (double)segs;
    for (int64_t i = 0; i <= segs; i++) {
        double a = (double)i * step;
        iks_gl_vertex(cx + cos(a) * r, cy + sin(a) * r, 0);
    }
    iks_gl_end();
}

static void iks_gl_line(double x1, double y1, double x2, double y2) {
    iks_gl_begin(1);
    iks_gl_vertex(x1, y1, 0);
    iks_gl_vertex(x2, y2, 0);
    iks_gl_end();
}
static void iks_gl_point_size(double s) { glPointSize((GLfloat)s); }
static void iks_gl_line_width(double w) { glLineWidth((GLfloat)w); }

static void iks_gl_depth(int64_t on) {
    if (on) glEnable(GL_DEPTH_TEST);
    else glDisable(GL_DEPTH_TEST);
}

static void iks_gl_blend(int64_t on) {
    if (on) {
        glEnable(GL_BLEND);
        glBlendFunc(GL_ONE, GL_ONE);
    } else {
        glDisable(GL_BLEND);
    }
}

static void iks_gl_proj_ortho(double l, double r, double b, double t) {
    glMatrixMode(GL_PROJECTION);
    glLoadIdentity();
    glOrtho(l, r, b, t, -1.0, 1.0);
    glMatrixMode(GL_MODELVIEW);
}

static void iks_gl_title(const char* t) {
    if (iks_hwnd) SetWindowTextA(iks_hwnd, t ? t : "IKS");
}

static void iks_gl_fullscreen(int64_t on) {
    static DWORD saved_style = 0;
    static RECT saved_rect;
    static int is_fs = 0;
    if (!iks_hwnd) return;
    if (on && !is_fs) {
        saved_style = GetWindowLongA(iks_hwnd, GWL_STYLE);
        GetWindowRect(iks_hwnd, &saved_rect);
        SetWindowLongA(iks_hwnd, GWL_STYLE, WS_POPUP | WS_VISIBLE);
        MONITORINFO mi;
        mi.cbSize = sizeof(mi);
        GetMonitorInfoA(MonitorFromWindow(iks_hwnd, MONITOR_DEFAULTTONEAREST), &mi);
        SetWindowPos(iks_hwnd, HWND_TOP, mi.rcMonitor.left, mi.rcMonitor.top,
                     mi.rcMonitor.right - mi.rcMonitor.left,
                     mi.rcMonitor.bottom - mi.rcMonitor.top,
                     SWP_FRAMECHANGED | SWP_NOOWNERZORDER);
        is_fs = 1;
    } else if (!on && is_fs) {
        SetWindowLongA(iks_hwnd, GWL_STYLE, saved_style);
        SetWindowPos(iks_hwnd, HWND_TOP, saved_rect.left, saved_rect.top,
                     saved_rect.right - saved_rect.left, saved_rect.bottom - saved_rect.top,
                     SWP_FRAMECHANGED | SWP_NOOWNERZORDER);
        is_fs = 0;
    }
}

static int64_t iks_gl_width(void) {
    RECT r;
    if (!iks_hwnd) return 0;
    GetClientRect(iks_hwnd, &r);
    return r.right - r.left;
}

static int64_t iks_gl_height(void) {
    RECT r;
    if (!iks_hwnd) return 0;
    GetClientRect(iks_hwnd, &r);
    return r.bottom - r.top;
}

static int64_t iks_gl_swap(void) {
    if (!iks_glrc) return 0;
    SwapBuffers(iks_gldc);
    iks_gl_swap_query_end();
    return iks_gfx_poll();
}

/* --- D3D9 backend --- */
static IDirect3D9* iks_d9d3d;
static IDirect3DDevice9* iks_d9dev;
static int iks_d9on;
static float iks_d9cr, iks_d9cg, iks_d9cb;
typedef struct { float x, y, z, r, g, b; } IksD9V;
static IksD9V* iks_d9vv;
static int iks_d9vn, iks_d9vcap;
static int iks_d9mode, iks_d9in_begin;

static void iks_d9_fail(HRESULT hr, const char* what) {
    fprintf(stderr, "d3d9 error: %s (hr=0x%08lx)\n", what, (unsigned long)hr);
    exit(1);
}

static void iks_d9_init(int64_t w, int64_t h, const char* title) {
    if (iks_d9on) return;
    if (!iks_gfx_on) iks_gfx_open(w, h, title);
    iks_d9d3d = Direct3DCreate9(D3D_SDK_VERSION);
    if (!iks_d9d3d) { fprintf(stderr, "d3d9 error: Direct3DCreate9 failed\n"); exit(1); }
    D3DPRESENT_PARAMETERS pp;
    ZeroMemory(&pp, sizeof(pp));
    pp.Windowed = TRUE;
    pp.SwapEffect = D3DSWAPEFFECT_DISCARD;
    pp.BackBufferFormat = D3DFMT_UNKNOWN;
    pp.hDeviceWindow = iks_hwnd;
    pp.PresentationInterval = D3DPRESENT_INTERVAL_ONE;
    HRESULT hr = IDirect3D9_CreateDevice(iks_d9d3d, D3DADAPTER_DEFAULT, D3DDEVTYPE_HAL, iks_hwnd,
        D3DCREATE_HARDWARE_VERTEXPROCESSING, &pp, &iks_d9dev);
    if (FAILED(hr))
        hr = IDirect3D9_CreateDevice(iks_d9d3d, D3DADAPTER_DEFAULT, D3DDEVTYPE_HAL, iks_hwnd,
            D3DCREATE_SOFTWARE_VERTEXPROCESSING, &pp, &iks_d9dev);
    if (FAILED(hr)) iks_d9_fail(hr, "CreateDevice");
    /* fixed function: no lighting, vertex colors */
    IDirect3DDevice9_SetRenderState(iks_d9dev, D3DRS_LIGHTING, FALSE);
    IDirect3DDevice9_SetRenderState(iks_d9dev, D3DRS_CULLMODE, D3DCULL_NONE);
    IDirect3DDevice9_SetRenderState(iks_d9dev, D3DRS_ALPHABLENDENABLE, TRUE);
    IDirect3DDevice9_SetRenderState(iks_d9dev, D3DRS_SRCBLEND, D3DBLEND_SRCALPHA);
    IDirect3DDevice9_SetRenderState(iks_d9dev, D3DRS_DESTBLEND, D3DBLEND_INVSRCALPHA);
    IDirect3DDevice9_SetFVF(iks_d9dev, D3DFVF_XYZ | D3DFVF_DIFFUSE);
    iks_d9on = 1;
}

static void iks_d9_clear(int64_t r, int64_t g, int64_t b) {
    if (!iks_d9on) { fprintf(stderr, "d3d9 error: call d3d9_init() first\n"); exit(1); }
    D3DCOLOR c = D3DCOLOR_XRGB((BYTE)iks_clamp8(r), (BYTE)iks_clamp8(g), (BYTE)iks_clamp8(b));
    IDirect3DDevice9_Clear(iks_d9dev, 0, NULL, D3DCLEAR_TARGET, c, 1.0f, 0);
}

static void iks_d9_color(int64_t r, int64_t g, int64_t b) {
    iks_d9cr = (float)iks_clamp8(r) / 255.0f;
    iks_d9cg = (float)iks_clamp8(g) / 255.0f;
    iks_d9cb = (float)iks_clamp8(b) / 255.0f;
}

static void iks_d9_begin(int64_t mode) {
    if (!iks_d9on) { fprintf(stderr, "d3d9 error: call d3d9_init() first\n"); exit(1); }
    iks_d9mode = (mode == 1) ? 1 : 0;
    iks_d9in_begin = 1;
}

static void iks_d9_vertex(double x, double y, double z) {
    if (!iks_d9in_begin) return;
    if (iks_d9vn >= iks_d9vcap) {
        int nc = iks_d9vcap ? iks_d9vcap * 2 : 4096;
        IksD9V* nb = (IksD9V*)realloc(iks_d9vv, (size_t)nc * sizeof(IksD9V));
        if (!nb) { fprintf(stderr, "out of memory\n"); exit(1); }
        iks_d9vv = nb;
        iks_d9vcap = nc;
    }
    IksD9V* v = &iks_d9vv[iks_d9vn++];
    v->x = (float)x;
    v->y = (float)y;
    v->z = (float)z;
    /* pack color as D3DCOLOR (ABGR in memory for XYZRHW-style, but DIFFUSE is ARGB) */
    BYTE rr = (BYTE)(iks_d9cr * 255.0f), gg = (BYTE)(iks_d9cg * 255.0f), bb = (BYTE)(iks_d9cb * 255.0f);
    v->r = 0.0f;
    /* we'll use a separate color array approach: encode in z? No — use a different struct */
    /* Actually D3DFVF_XYZ | D3DFVF_DIFFUSE expects DWORD diffuse after xyz.
       Let's repurpose: store packed color in the float r field's bits. */
    v->r = (float)((DWORD)(0xFFu << 24) | ((DWORD)bb << 16) | ((DWORD)gg << 8) | (DWORD)rr);
    v->g = 0.0f;
    v->b = 0.0f;
}

static void iks_d9_end(void) {
    if (!iks_d9in_begin) return;
    iks_d9in_begin = 0;
    if (iks_d9vn <= 0) return;
    IDirect3DDevice9_BeginScene(iks_d9dev);
    /* We stored: x,y,z (floats) then packed color in r field.
       But FVF layout expects xyz then DWORD. Our struct is xyz rgb (24 bytes).
       FVF XYZ|DIFFUSE = 12 + 4 = 16 bytes. Mismatch.
       Better: build a temp buffer with correct layout. */
    typedef struct { float x, y, z; DWORD col; } D9V;
    D9V* tmp = (D9V*)malloc((size_t)iks_d9vn * sizeof(D9V));
    for (int i = 0; i < iks_d9vn; i++) {
        tmp[i].x = iks_d9vv[i].x;
        tmp[i].y = iks_d9vv[i].y;
        tmp[i].z = iks_d9vv[i].z;
        tmp[i].col = (DWORD)iks_d9vv[i].r;
    }
    D3DPRIMITIVETYPE pt = iks_d9mode ? D3DPT_LINELIST : D3DPT_TRIANGLELIST;
    UINT pcount = iks_d9mode ? (UINT)(iks_d9vn / 2) : (UINT)(iks_d9vn / 3);
    IDirect3DDevice9_DrawPrimitiveUP(iks_d9dev, pt, pcount, tmp, sizeof(D9V));
    free(tmp);
    IDirect3DDevice9_EndScene(iks_d9dev);
    iks_d9vn = 0;
}

static void iks_d9_rect(double x, double y, double w, double h) {
    iks_d9_begin(3);
    iks_d9_vertex(x, y + h, 0.0);
    iks_d9_vertex(x + w, y, 0.0);
    iks_d9_vertex(x, y, 0.0);
    iks_d9_vertex(x, y + h, 0.0);
    iks_d9_vertex(x + w, y + h, 0.0);
    iks_d9_vertex(x + w, y, 0.0);
    iks_d9_end();
}

static void iks_d9_circle(double cx, double cy, double r, int64_t segs) {
    if (segs < 3) segs = 3;
    iks_d9_begin(3);
    for (int i = 0; i < segs; i++) {
        double a0 = (double)i / segs * 6.283185307179586;
        double a1 = (double)(i + 1) / segs * 6.283185307179586;
        iks_d9_vertex(cx, cy, 0.0);
        iks_d9_vertex(cx + r * cos(a0), cy + r * sin(a0), 0.0);
        iks_d9_vertex(cx + r * cos(a1), cy + r * sin(a1), 0.0);
    }
    iks_d9_end();
}

static void iks_d9_line(double x1, double y1, double x2, double y2) {
    iks_d9_begin(1);
    iks_d9_vertex(x1, y1, 0.0);
    iks_d9_vertex(x2, y2, 0.0);
    iks_d9_end();
}

static int64_t iks_d9_swap(void) {
    if (!iks_d9on) return 0;
    HRESULT hr = IDirect3DDevice9_Present(iks_d9dev, NULL, NULL, NULL, NULL);
    if (FAILED(hr) && hr != D3DERR_DEVICELOST) iks_d9_fail(hr, "Present");
    return iks_gfx_poll();
}

static void iks_d9_title(const char* t) {
    if (iks_hwnd) SetWindowTextA(iks_hwnd, t ? t : "IKS");
}

static int64_t iks_d9_width(void) { return iks_gw; }
static int64_t iks_d9_height(void) { return iks_gh; }

/* --- D3D10 backend --- */
static ID3D10Device* iks_d10dev;
static IDXGISwapChain* iks_d10sc;
static ID3D10RenderTargetView* iks_d10rtv;
static ID3D10VertexShader* iks_d10vs;
static ID3D10PixelShader* iks_d10ps;
static ID3D10InputLayout* iks_d10il;
static int iks_d10on;
static float iks_d10cr, iks_d10cg, iks_d10cb;
typedef struct { float x, y, z, r, g, b; } IksD10V;
static IksD10V* iks_d10vv;
static int iks_d10vn, iks_d10vcap;
static int iks_d10mode, iks_d10in_begin;

static void iks_d10_fail(HRESULT hr, const char* what) {
    fprintf(stderr, "d3d10 error: %s (hr=0x%08lx)\n", what, (unsigned long)hr);
    exit(1);
}

static const char* iks_d10_vs_src =
    "struct VS_IN { float3 pos: POSITION; float3 col: COLOR; };\n"
    "struct VS_OUT { float4 pos: SV_POSITION; float3 col: COLOR; };\n"
    "VS_OUT vs_main(VS_IN i) { VS_OUT o; o.pos = float4(i.pos, 1.0); o.col = i.col; return o; }\n";
static const char* iks_d10_ps_src =
    "struct VS_OUT { float4 pos: SV_POSITION; float3 col: COLOR; };\n"
    "float4 ps_main(VS_OUT i) : SV_TARGET { return float4(i.col, 1.0); }\n";

static void iks_d10_init(int64_t w, int64_t h, const char* title) {
    if (iks_d10on) return;
    if (!iks_gfx_on) iks_gfx_open(w, h, title);
    DXGI_SWAP_CHAIN_DESC sd;
    ZeroMemory(&sd, sizeof(sd));
    sd.BufferDesc.Width = iks_gw;
    sd.BufferDesc.Height = iks_gh;
    sd.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    sd.SampleDesc.Count = 1;
    sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    sd.BufferCount = 1;
    sd.OutputWindow = iks_hwnd;
    sd.Windowed = TRUE;
    sd.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;
    D3D10_FEATURE_LEVEL1 levels[1] = { D3D10_FEATURE_LEVEL_10_0 };
    D3D_FEATURE_LEVEL got;
    HRESULT hr = D3D10CreateDeviceAndSwapChain(NULL, D3D10_DRIVER_TYPE_HARDWARE, NULL, 0,
        D3D10_SDK_VERSION, &sd, &iks_d10sc, &iks_d10dev);
    if (FAILED(hr)) iks_d10_fail(hr, "D3D10CreateDeviceAndSwapChain");
    ID3D10Texture2D* back = NULL;
    hr = IDXGISwapChain_GetBuffer(iks_d10sc, 0, &IID_ID3D10Texture2D, (void**)&back);
    if (FAILED(hr)) iks_d10_fail(hr, "GetBuffer");
    hr = ID3D10Device_CreateRenderTargetView(iks_d10dev, (ID3D10Resource*)back, NULL, &iks_d10rtv);
    ID3D10Texture2D_Release(back);
    if (FAILED(hr)) iks_d10_fail(hr, "CreateRenderTargetView");
    /* shaders */
    ID3DBlob* vsb = NULL;
    ID3DBlob* psb = NULL;
    ID3DBlob* errb = NULL;
    hr = D3DCompile(iks_d10_vs_src, strlen(iks_d10_vs_src), NULL, NULL, NULL,
                    "vs_main", "vs_4_0", 0, 0, &vsb, &errb);
    if (FAILED(hr)) {
        fprintf(stderr, "d3d10 error: vs compile: %s\n", errb ? (char*)ID3D10Blob_GetBufferPointer(errb) : "?");
        exit(1);
    }
    hr = D3DCompile(iks_d10_ps_src, strlen(iks_d10_ps_src), NULL, NULL, NULL,
                    "ps_main", "ps_4_0", 0, 0, &psb, &errb);
    if (FAILED(hr)) {
        fprintf(stderr, "d3d10 error: ps compile: %s\n", errb ? (char*)ID3D10Blob_GetBufferPointer(errb) : "?");
        exit(1);
    }
    hr = ID3D10Device_CreateVertexShader(iks_d10dev, ID3D10Blob_GetBufferPointer(vsb),
                                         ID3D10Blob_GetBufferSize(vsb), &iks_d10vs);
    if (FAILED(hr)) iks_d10_fail(hr, "CreateVertexShader");
    hr = ID3D10Device_CreatePixelShader(iks_d10dev, ID3D10Blob_GetBufferPointer(psb),
                                        ID3D10Blob_GetBufferSize(psb), &iks_d10ps);
    if (FAILED(hr)) iks_d10_fail(hr, "CreatePixelShader");
    /* input layout */
    D3D10_INPUT_ELEMENT_DESC layout[2];
    layout[0].SemanticName = "POSITION";
    layout[0].SemanticIndex = 0;
    layout[0].Format = DXGI_FORMAT_R32G32B32_FLOAT;
    layout[0].InputSlot = 0;
    layout[0].AlignedByteOffset = 0;
    layout[0].InputSlotClass = D3D10_INPUT_PER_VERTEX_DATA;
    layout[0].InstanceDataStepRate = 0;
    layout[1].SemanticName = "COLOR";
    layout[1].SemanticIndex = 0;
    layout[1].Format = DXGI_FORMAT_R32G32B32_FLOAT;
    layout[1].InputSlot = 0;
    layout[1].AlignedByteOffset = 12;
    layout[1].InputSlotClass = D3D10_INPUT_PER_VERTEX_DATA;
    layout[1].InstanceDataStepRate = 0;
    hr = ID3D10Device_CreateInputLayout(iks_d10dev, layout, 2,
                                        ID3D10Blob_GetBufferPointer(vsb),
                                        ID3D10Blob_GetBufferSize(vsb), &iks_d10il);
    if (FAILED(hr)) iks_d10_fail(hr, "CreateInputLayout");
    ID3D10Blob_Release(vsb);
    ID3D10Blob_Release(psb);
    /* state */
    ID3D10Device_VSSetShader(iks_d10dev, iks_d10vs);
    ID3D10Device_PSSetShader(iks_d10dev, iks_d10ps);
    ID3D10Device_IASetInputLayout(iks_d10dev, iks_d10il);
    ID3D10Device_IASetPrimitiveTopology(iks_d10dev, D3D10_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    ID3D10Device_OMSetRenderTargets(iks_d10dev, 1, &iks_d10rtv, NULL);
    D3D10_VIEWPORT vp;
    vp.TopLeftX = 0;
    vp.TopLeftY = 0;
    vp.Width = iks_gw;
    vp.Height = iks_gh;
    vp.MinDepth = 0.0f;
    vp.MaxDepth = 1.0f;
    ID3D10Device_RSSetViewports(iks_d10dev, 1, &vp);
    /* blend state: enable alpha blending */
    D3D10_BLEND_DESC bd;
    ZeroMemory(&bd, sizeof(bd));
    bd.AlphaToCoverageEnable = FALSE;
    bd.BlendEnable[0] = TRUE;
    bd.SrcBlend = D3D10_BLEND_SRC_ALPHA;
    bd.DestBlend = D3D10_BLEND_INV_SRC_ALPHA;
    bd.BlendOp = D3D10_BLEND_OP_ADD;
    bd.SrcBlendAlpha = D3D10_BLEND_ONE;
    bd.DestBlendAlpha = D3D10_BLEND_ZERO;
    bd.BlendOpAlpha = D3D10_BLEND_OP_ADD;
    bd.RenderTargetWriteMask[0] = D3D10_COLOR_WRITE_ENABLE_ALL;
    ID3D10BlendState* bs = NULL;
    hr = ID3D10Device_CreateBlendState(iks_d10dev, &bd, &bs);
    if (SUCCEEDED(hr)) {
        float factor[4] = {0,0,0,0};
        ID3D10Device_OMSetBlendState(iks_d10dev, bs, factor, 0xffffffff);
        ID3D10BlendState_Release(bs);
    }
    /* rasterizer: no cull */
    D3D10_RASTERIZER_DESC rd;
    ZeroMemory(&rd, sizeof(rd));
    rd.FillMode = D3D10_FILL_SOLID;
    rd.CullMode = D3D10_CULL_NONE;
    rd.FrontCounterClockwise = FALSE;
    rd.DepthClipEnable = TRUE;
    ID3D10RasterizerState* rs = NULL;
    hr = ID3D10Device_CreateRasterizerState(iks_d10dev, &rd, &rs);
    if (SUCCEEDED(hr)) {
        ID3D10Device_RSSetState(iks_d10dev, rs);
        ID3D10RasterizerState_Release(rs);
    }
    iks_d10on = 1;
}

static void iks_d10_clear(int64_t r, int64_t g, int64_t b) {
    if (!iks_d10on) { fprintf(stderr, "d3d10 error: call dx10_init() first\n"); exit(1); }
    float c[4];
    c[0] = (float)iks_clamp8(r) / 255.0f;
    c[1] = (float)iks_clamp8(g) / 255.0f;
    c[2] = (float)iks_clamp8(b) / 255.0f;
    c[3] = 1.0f;
    ID3D10Device_ClearRenderTargetView(iks_d10dev, iks_d10rtv, c);
}

static void iks_d10_color(int64_t r, int64_t g, int64_t b) {
    iks_d10cr = (float)iks_clamp8(r) / 255.0f;
    iks_d10cg = (float)iks_clamp8(g) / 255.0f;
    iks_d10cb = (float)iks_clamp8(b) / 255.0f;
}

static void iks_d10_begin(int64_t mode) {
    if (!iks_d10on) { fprintf(stderr, "d3d10 error: call dx10_init() first\n"); exit(1); }
    iks_d10mode = (mode == 1) ? 1 : 0;
    iks_d10in_begin = 1;
}

static void iks_d10_vertex(double x, double y, double z) {
    if (!iks_d10in_begin) return;
    if (iks_d10vn >= iks_d10vcap) {
        int nc = iks_d10vcap ? iks_d10vcap * 2 : 4096;
        IksD10V* nb = (IksD10V*)realloc(iks_d10vv, (size_t)nc * sizeof(IksD10V));
        if (!nb) { fprintf(stderr, "out of memory\n"); exit(1); }
        iks_d10vv = nb;
        iks_d10vcap = nc;
    }
    IksD10V* v = &iks_d10vv[iks_d10vn++];
    v->x = (float)x;
    v->y = (float)y;
    v->z = (float)z;
    v->r = iks_d10cr;
    v->g = iks_d10cg;
    v->b = iks_d10cb;
}

static void iks_d10_end(void) {
    if (!iks_d10in_begin) return;
    iks_d10in_begin = 0;
    if (iks_d10vn <= 0) return;
    UINT bytes = (UINT)(iks_d10vn * sizeof(IksD10V));
    D3D10_BUFFER_DESC bd;
    ZeroMemory(&bd, sizeof(bd));
    bd.ByteWidth = bytes;
    bd.Usage = D3D10_USAGE_DEFAULT;
    bd.BindFlags = D3D10_BIND_VERTEX_BUFFER;
    D3D10_SUBRESOURCE_DATA sd;
    sd.pSysMem = iks_d10vv;
    sd.SysMemPitch = 0;
    sd.SysMemSlicePitch = 0;
    ID3D10Buffer* vb = NULL;
    HRESULT hr = ID3D10Device_CreateBuffer(iks_d10dev, &bd, &sd, &vb);
    if (FAILED(hr)) iks_d10_fail(hr, "CreateBuffer");
    UINT stride = sizeof(IksD10V);
    UINT offset = 0;
    ID3D10Device_IASetVertexBuffers(iks_d10dev, 0, 1, &vb, &stride, &offset);
    D3D10_PRIMITIVE_TOPOLOGY topo = iks_d10mode ? D3D10_PRIMITIVE_TOPOLOGY_LINELIST : D3D10_PRIMITIVE_TOPOLOGY_TRIANGLELIST;
    ID3D10Device_IASetPrimitiveTopology(iks_d10dev, topo);
    UINT pcount = iks_d10mode ? (UINT)(iks_d10vn / 2) : (UINT)(iks_d10vn / 3);
    ID3D10Device_Draw(iks_d10dev, pcount, 0);
    ID3D10Buffer_Release(vb);
    ID3D10Device_IASetPrimitiveTopology(iks_d10dev, D3D10_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    iks_d10vn = 0;
}

static void iks_d10_rect(double x, double y, double w, double h) {
    iks_d10_begin(3);
    iks_d10_vertex(x, y + h, 0.0);
    iks_d10_vertex(x + w, y, 0.0);
    iks_d10_vertex(x, y, 0.0);
    iks_d10_vertex(x, y + h, 0.0);
    iks_d10_vertex(x + w, y + h, 0.0);
    iks_d10_vertex(x + w, y, 0.0);
    iks_d10_end();
}

static void iks_d10_circle(double cx, double cy, double r, int64_t segs) {
    if (segs < 3) segs = 3;
    iks_d10_begin(3);
    for (int i = 0; i < segs; i++) {
        double a0 = (double)i / segs * 6.283185307179586;
        double a1 = (double)(i + 1) / segs * 6.283185307179586;
        iks_d10_vertex(cx, cy, 0.0);
        iks_d10_vertex(cx + r * cos(a0), cy + r * sin(a0), 0.0);
        iks_d10_vertex(cx + r * cos(a1), cy + r * sin(a1), 0.0);
    }
    iks_d10_end();
}

static void iks_d10_line(double x1, double y1, double x2, double y2) {
    iks_d10_begin(1);
    iks_d10_vertex(x1, y1, 0.0);
    iks_d10_vertex(x2, y2, 0.0);
    iks_d10_end();
}

static int64_t iks_d10_swap(void) {
    if (!iks_d10on) return 0;
    IDXGISwapChain_Present(iks_d10sc, 0, 0);
    return iks_gfx_poll();
}

static void iks_d10_title(const char* t) {
    if (iks_hwnd) SetWindowTextA(iks_hwnd, t ? t : "IKS");
}

static int64_t iks_d10_width(void) { return iks_gw; }
static int64_t iks_d10_height(void) { return iks_gh; }

static const char* iks_dvs_src =
    "struct VS_IN { float3 pos: POSITION; float3 col: COLOR; };\n"
    "struct VS_OUT { float4 pos: SV_POSITION; float3 col: COLOR; };\n"
    "VS_OUT vs_main(VS_IN i) { VS_OUT o; o.pos = float4(i.pos, 1.0); o.col = i.col; return o; }\n";

static const char* iks_dps_src =
    "struct VS_OUT { float4 pos: SV_POSITION; float3 col: COLOR; };\n"
    "float4 ps_main(VS_OUT i) : SV_TARGET { return float4(i.col, 1.0); }\n";

static void iks_d3d_fail(HRESULT hr, const char* what) {
    fprintf(stderr, "d3d11 error: %s (hr=0x%08lx)\n", what, (unsigned long)hr);
    exit(1);
}

static void iks_d3d_init(int64_t w, int64_t h, const char* title) {
    if (!iks_gfx_on) iks_gfx_open(w, h, title);
    if (iks_ddev) return;
    DXGI_SWAP_CHAIN_DESC sd;
    ZeroMemory(&sd, sizeof(sd));
    sd.BufferDesc.Width = iks_gw;
    sd.BufferDesc.Height = iks_gh;
    sd.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    sd.SampleDesc.Count = 1;
    sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    sd.BufferCount = 1;
    sd.OutputWindow = iks_hwnd;
    sd.Windowed = TRUE;
    sd.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;
    D3D_FEATURE_LEVEL levels[3];
    levels[0] = D3D_FEATURE_LEVEL_11_0;
    levels[1] = D3D_FEATURE_LEVEL_10_1;
    levels[2] = D3D_FEATURE_LEVEL_10_0;
    D3D_FEATURE_LEVEL got;
    HRESULT hr = D3D11CreateDeviceAndSwapChain(
        NULL, D3D_DRIVER_TYPE_HARDWARE, NULL, 0, levels, 3,
        D3D11_SDK_VERSION, &sd, &iks_dsc, &iks_ddev, &got, &iks_dctx);
    if (FAILED(hr)) iks_d3d_fail(hr, "D3D11CreateDeviceAndSwapChain");
    ID3D11Texture2D* back = NULL;
    hr = IDXGISwapChain_GetBuffer(iks_dsc, 0, &IID_ID3D11Texture2D, (void**)&back);
    if (FAILED(hr)) iks_d3d_fail(hr, "GetBuffer");
    hr = ID3D11Device_CreateRenderTargetView(iks_ddev, (ID3D11Resource*)back, NULL, &iks_drv);
    ID3D11Texture2D_Release(back);
    if (FAILED(hr)) iks_d3d_fail(hr, "CreateRenderTargetView");
    ID3DBlob* vsb = NULL;
    ID3DBlob* psb = NULL;
    ID3DBlob* errb = NULL;
    hr = D3DCompile(iks_dvs_src, strlen(iks_dvs_src), NULL, NULL, NULL,
                    "vs_main", "vs_4_0", 0, 0, &vsb, &errb);
    if (FAILED(hr)) {
        fprintf(stderr, "d3d11 error: vertex shader compile failed: %s\n",
                errb ? (char*)ID3D10Blob_GetBufferPointer(errb) : "?");
        exit(1);
    }
    hr = D3DCompile(iks_dps_src, strlen(iks_dps_src), NULL, NULL, NULL,
                    "ps_main", "ps_4_0", 0, 0, &psb, &errb);
    if (FAILED(hr)) {
        fprintf(stderr, "d3d11 error: pixel shader compile failed: %s\n",
                errb ? (char*)ID3D10Blob_GetBufferPointer(errb) : "?");
        exit(1);
    }
    hr = ID3D11Device_CreateVertexShader(iks_ddev, ID3D10Blob_GetBufferPointer(vsb),
                                         ID3D10Blob_GetBufferSize(vsb), NULL, &iks_dvs);
    if (FAILED(hr)) iks_d3d_fail(hr, "CreateVertexShader");
    hr = ID3D11Device_CreatePixelShader(iks_ddev, ID3D10Blob_GetBufferPointer(psb),
                                        ID3D10Blob_GetBufferSize(psb), NULL, &iks_dps);
    if (FAILED(hr)) iks_d3d_fail(hr, "CreatePixelShader");
    D3D11_INPUT_ELEMENT_DESC layout[2];
    layout[0].SemanticName = "POSITION";
    layout[0].SemanticIndex = 0;
    layout[0].Format = DXGI_FORMAT_R32G32B32_FLOAT;
    layout[0].InputSlot = 0;
    layout[0].AlignedByteOffset = 0;
    layout[0].InputSlotClass = D3D11_INPUT_PER_VERTEX_DATA;
    layout[0].InstanceDataStepRate = 0;
    layout[1].SemanticName = "COLOR";
    layout[1].SemanticIndex = 0;
    layout[1].Format = DXGI_FORMAT_R32G32B32_FLOAT;
    layout[1].InputSlot = 0;
    layout[1].AlignedByteOffset = 12;
    layout[1].InputSlotClass = D3D11_INPUT_PER_VERTEX_DATA;
    layout[1].InstanceDataStepRate = 0;
    hr = ID3D11Device_CreateInputLayout(iks_ddev, layout, 2,
                                        ID3D10Blob_GetBufferPointer(vsb),
                                        ID3D10Blob_GetBufferSize(vsb), &iks_dil);
    if (FAILED(hr)) iks_d3d_fail(hr, "CreateInputLayout");
    ID3D10Blob_Release(vsb);
    ID3D10Blob_Release(psb);
    ID3D11DeviceContext_VSSetShader(iks_dctx, iks_dvs, NULL, 0);
    ID3D11DeviceContext_PSSetShader(iks_dctx, iks_dps, NULL, 0);
    ID3D11DeviceContext_IASetInputLayout(iks_dctx, iks_dil);
    ID3D11DeviceContext_IASetPrimitiveTopology(iks_dctx, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    ID3D11DeviceContext_OMSetRenderTargets(iks_dctx, 1, &iks_drv, NULL);
    D3D11_VIEWPORT vp;
    vp.TopLeftX = 0.0f;
    vp.TopLeftY = 0.0f;
    vp.Width = (FLOAT)iks_gw;
    vp.Height = (FLOAT)iks_gh;
    vp.MinDepth = 0.0f;
    vp.MaxDepth = 1.0f;
    ID3D11DeviceContext_RSSetViewports(iks_dctx, 1, &vp);
}

static void iks_d3d_clear(int64_t r, int64_t g, int64_t b) {
    if (!iks_ddev) {
        fprintf(stderr, "d3d11 error: call d3d_init() first\n");
        exit(1);
    }
    float c[4];
    c[0] = (float)iks_clamp8(r) / 255.0f;
    c[1] = (float)iks_clamp8(g) / 255.0f;
    c[2] = (float)iks_clamp8(b) / 255.0f;
    c[3] = 1.0f;
    ID3D11DeviceContext_ClearRenderTargetView(iks_dctx, iks_drv, c);
}

static void iks_d3d_draw(IksArray a) {
    if (!iks_ddev) {
        fprintf(stderr, "d3d11 error: call d3d_init() first\n");
        exit(1);
    }
    if (a.tag != IKS_T_FLOAT || a.len % 6 != 0) {
        fprintf(stderr, "d3d11 error: d3d_draw expects a float[] with 6 values per vertex (x,y,z,r,g,b)\n");
        exit(1);
    }
    if (a.len == 0) return;
    D3D11_BUFFER_DESC bd;
    ZeroMemory(&bd, sizeof(bd));
    bd.ByteWidth = (UINT)(a.len * sizeof(double));
    bd.Usage = D3D11_USAGE_DEFAULT;
    bd.BindFlags = D3D11_BIND_VERTEX_BUFFER;
    D3D11_SUBRESOURCE_DATA sd;
    sd.pSysMem = a.data;
    sd.SysMemPitch = 0;
    sd.SysMemSlicePitch = 0;
    ID3D11Buffer* vb = NULL;
    HRESULT hr = ID3D11Device_CreateBuffer(iks_ddev, &bd, &sd, &vb);
    if (FAILED(hr)) iks_d3d_fail(hr, "CreateBuffer");
    UINT stride = 24;
    UINT offset = 0;
    ID3D11DeviceContext_IASetVertexBuffers(iks_dctx, 0, 1, &vb, &stride, &offset);
    ID3D11DeviceContext_Draw(iks_dctx, (UINT)(a.len / 6), 0);
    ID3D11Buffer_Release(vb);
}

static int64_t iks_d3d_present(void) {
    if (!iks_dsc) return 0;
    IDXGISwapChain_Present(iks_dsc, 0, 0);
    return iks_gfx_poll();
}

/* --- immediate mode state (mirrors gl and vk) --- */
typedef struct { float x, y, z, r, g, b; } IksD3DV;
static float iks_dcr = 1.0f, iks_dcg = 1.0f, iks_dcb = 1.0f;
static IksD3DV* iks_dvv = NULL;
static int iks_dvn = 0, iks_dvcap = 0;
static int iks_dmode = 0, iks_din_begin = 0;
static ID3D11Buffer* iks_dvbuf = NULL;
static UINT iks_dvbufsz = 0;

static void iks_d3d_color(int64_t r, int64_t g, int64_t b) {
    iks_dcr = (float)iks_clamp8(r) / 255.0f;
    iks_dcg = (float)iks_clamp8(g) / 255.0f;
    iks_dcb = (float)iks_clamp8(b) / 255.0f;
}

static void iks_d3d_begin(int64_t mode) {
    if (!iks_ddev) {
        fprintf(stderr, "d3d11 error: call d3d_init() first\n");
        exit(1);
    }
    iks_dmode = (mode == 1) ? 1 : 0; /* 1=lines else triangles */
    iks_din_begin = 1;
}

static void iks_d3d_vertex(double x, double y, double z) {
    if (!iks_din_begin) return;
    if (iks_dvn >= iks_dvcap) {
        int nc = iks_dvcap ? iks_dvcap * 2 : 4096;
        IksD3DV* nb = (IksD3DV*)realloc(iks_dvv, (size_t)nc * sizeof(IksD3DV));
        if (!nb) { fprintf(stderr, "out of memory\n"); exit(1); }
        iks_dvv = nb;
        iks_dvcap = nc;
    }
    IksD3DV* v = &iks_dvv[iks_dvn++];
    v->x = (float)x;
    v->y = (float)y;
    v->z = (float)z;
    v->r = iks_dcr;
    v->g = iks_dcg;
    v->b = iks_dcb;
}

static void iks_d3d_end(void) {
    if (!iks_din_begin) return;
    iks_din_begin = 0;
    if (iks_dvn <= 0) return;
    UINT bytes = (UINT)(iks_dvn * sizeof(IksD3DV));
    /* create or resize dynamic vertex buffer */
    if (!iks_dvbuf || bytes > iks_dvbufsz) {
        if (iks_dvbuf) ID3D11Buffer_Release(iks_dvbuf);
        UINT cap = bytes < 65536 ? 65536 : bytes;
        D3D11_BUFFER_DESC bd;
        ZeroMemory(&bd, sizeof(bd));
        bd.ByteWidth = cap;
        bd.Usage = D3D11_USAGE_DYNAMIC;
        bd.BindFlags = D3D11_BIND_VERTEX_BUFFER;
        bd.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
        HRESULT hr = ID3D11Device_CreateBuffer(iks_ddev, &bd, NULL, &iks_dvbuf);
        if (FAILED(hr)) iks_d3d_fail(hr, "CreateBuffer dynamic");
        iks_dvbufsz = cap;
    }
    D3D11_MAPPED_SUBRESOURCE ms;
    HRESULT hr = ID3D11DeviceContext_Map(iks_dctx, (ID3D11Resource*)iks_dvbuf, 0, D3D11_MAP_WRITE_DISCARD, 0, &ms);
    if (FAILED(hr)) iks_d3d_fail(hr, "Map");
    memcpy(ms.pData, iks_dvv, bytes);
    ID3D11DeviceContext_Unmap(iks_dctx, (ID3D11Resource*)iks_dvbuf, 0);
    UINT stride = sizeof(IksD3DV);
    UINT offset = 0;
    ID3D11DeviceContext_IASetVertexBuffers(iks_dctx, 0, 1, &iks_dvbuf, &stride, &offset);
    D3D11_PRIMITIVE_TOPOLOGY topo = iks_dmode ? D3D11_PRIMITIVE_TOPOLOGY_LINELIST : D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST;
    ID3D11DeviceContext_IASetPrimitiveTopology(iks_dctx, topo);
    ID3D11DeviceContext_Draw(iks_dctx, (UINT)iks_dvn, 0);
    /* restore default topology */
    ID3D11DeviceContext_IASetPrimitiveTopology(iks_dctx, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    iks_dvn = 0;
}

static void iks_d3d_rect(double x, double y, double w, double h) {
    iks_d3d_begin(3);
    iks_d3d_vertex(x, y + h, 0.0);
    iks_d3d_vertex(x + w, y, 0.0);
    iks_d3d_vertex(x, y, 0.0);
    iks_d3d_vertex(x, y + h, 0.0);
    iks_d3d_vertex(x + w, y + h, 0.0);
    iks_d3d_vertex(x + w, y, 0.0);
    iks_d3d_end();
}

static void iks_d3d_circle(double cx, double cy, double r, int64_t segs) {
    if (segs < 3) segs = 3;
    iks_d3d_begin(3);
    for (int i = 0; i < segs; i++) {
        double a0 = (double)i / segs * 6.283185307179586;
        double a1 = (double)(i + 1) / segs * 6.283185307179586;
        iks_d3d_vertex(cx, cy, 0.0);
        iks_d3d_vertex(cx + r * cos(a0), cy + r * sin(a0), 0.0);
        iks_d3d_vertex(cx + r * cos(a1), cy + r * sin(a1), 0.0);
    }
    iks_d3d_end();
}

static void iks_d3d_line(double x1, double y1, double x2, double y2) {
    iks_d3d_begin(1);
    iks_d3d_vertex(x1, y1, 0.0);
    iks_d3d_vertex(x2, y2, 0.0);
    iks_d3d_end();
}

static void iks_d3d_title(const char* t) {
    if (iks_hwnd) SetWindowTextA(iks_hwnd, t ? t : "IKS");
}

static int64_t iks_d3d_width(void) { return iks_gw; }
static int64_t iks_d3d_height(void) { return iks_gh; }

/* --- D3D12 backend --- */
#define IKS_D12_FRAMES 2
static ID3D12Device* iks_d12dev;
static ID3D12CommandQueue* iks_d12q;
static ID3D12CommandAllocator* iks_d12ca[IKS_D12_FRAMES];
static ID3D12GraphicsCommandList* iks_d12cl;
static IDXGISwapChain3* iks_d12sc;
static ID3D12DescriptorHeap* iks_d12rh; /* RTV heap */
static ID3D12Resource* iks_d12rt[IKS_D12_FRAMES];
static ID3D12Fence* iks_d12fence;
static HANDLE iks_d12fev;
static UINT64 iks_d12fval;
static UINT iks_d12fi; /* frame index */
static ID3D12RootSignature* iks_d12rs;
static ID3D12PipelineState* iks_d12ps_tri;
static ID3D12PipelineState* iks_d12ps_line;
static ID3D12Resource* iks_d12vb;
static void* iks_d12vbm;
static UINT64 iks_d12vbcap;
static UINT iks_d12rtinc;
static SIZE_T iks_d12rtv_base;
static int iks_d12on;
static int iks_d12open;
typedef struct { float x, y, z, r, g, b; } IksD12V;
static float iks_d12cr, iks_d12cg, iks_d12cb;
static IksD12V* iks_d12vv;
static int iks_d12vn, iks_d12vcap;
static int iks_d12mode, iks_d12in_begin;

static void iks_d12_fail(HRESULT hr, const char* what) {
    fprintf(stderr, "d3d12 error: %s (hr=0x%08lx)\n", what, (unsigned long)hr);
    exit(1);
}

static D3D12_CPU_DESCRIPTOR_HANDLE iks_d12_rtv(UINT fi) {
    D3D12_CPU_DESCRIPTOR_HANDLE h;
    iks_d12rh->lpVtbl->GetCPUDescriptorHandleForHeapStart(iks_d12rh, &h);
    h.ptr += (SIZE_T)fi * iks_d12rtinc;
    return h;
}

static const char* iks_d12_vs_src =
    "struct VS_IN { float3 pos: POSITION; float3 col: COLOR; };\n"
    "struct VS_OUT { float4 pos: SV_POSITION; float3 col: COLOR; };\n"
    "VS_OUT vs_main(VS_IN i) { VS_OUT o; o.pos = float4(i.pos, 1.0); o.col = i.col; return o; }\n";
static const char* iks_d12_ps_src =
    "struct VS_OUT { float4 pos: SV_POSITION; float3 col: COLOR; };\n"
    "float4 ps_main(VS_OUT i) : SV_TARGET { return float4(i.col, 1.0); }\n";

static void iks_d12_make_pipeline(void) {
    ID3DBlob* vsb = NULL;
    ID3DBlob* psb = NULL;
    ID3DBlob* errb = NULL;
    HRESULT hr = D3DCompile(iks_d12_vs_src, strlen(iks_d12_vs_src), NULL, NULL, NULL,
                            "vs_main", "vs_4_0", 0, 0, &vsb, &errb);
    if (FAILED(hr)) {
        fprintf(stderr, "d3d12 error: vs compile: %s\n", errb ? (char*)ID3D10Blob_GetBufferPointer(errb) : "?");
        exit(1);
    }
    hr = D3DCompile(iks_d12_ps_src, strlen(iks_d12_ps_src), NULL, NULL, NULL,
                    "ps_main", "ps_4_0", 0, 0, &psb, &errb);
    if (FAILED(hr)) {
        fprintf(stderr, "d3d12 error: ps compile: %s\n", errb ? (char*)ID3D10Blob_GetBufferPointer(errb) : "?");
        exit(1);
    }
    /* root signature (empty) */
    D3D12_ROOT_SIGNATURE_DESC rsd;
    ZeroMemory(&rsd, sizeof(rsd));
    rsd.NumParameters = 0;
    rsd.NumStaticSamplers = 0;
    ID3DBlob* rsb = NULL;
    hr = D3D12SerializeRootSignature(&rsd, D3D_ROOT_SIGNATURE_VERSION_1, &rsb, &errb);
    if (FAILED(hr)) iks_d12_fail(hr, "SerializeRootSignature");
    hr = ID3D12Device_CreateRootSignature(iks_d12dev, 0,
        ID3D10Blob_GetBufferPointer(rsb), ID3D10Blob_GetBufferSize(rsb),
        &IID_ID3D12RootSignature, (void**)&iks_d12rs);
    if (FAILED(hr)) iks_d12_fail(hr, "CreateRootSignature");
    ID3D10Blob_Release(rsb);
    /* input layout */
    D3D12_INPUT_ELEMENT_DESC layout[2];
    layout[0].SemanticName = "POSITION";
    layout[0].SemanticIndex = 0;
    layout[0].Format = DXGI_FORMAT_R32G32B32_FLOAT;
    layout[0].InputSlot = 0;
    layout[0].AlignedByteOffset = 0;
    layout[0].InputSlotClass = D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA;
    layout[0].InstanceDataStepRate = 0;
    layout[1].SemanticName = "COLOR";
    layout[1].SemanticIndex = 0;
    layout[1].Format = DXGI_FORMAT_R32G32B32_FLOAT;
    layout[1].InputSlot = 0;
    layout[1].AlignedByteOffset = 12;
    layout[1].InputSlotClass = D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA;
    layout[1].InstanceDataStepRate = 0;
    /* pipeline state x2 (tri / line) */
    for (int i = 0; i < 2; i++) {
        D3D12_GRAPHICS_PIPELINE_STATE_DESC pd;
        ZeroMemory(&pd, sizeof(pd));
        pd.pRootSignature = iks_d12rs;
        pd.VS.pShaderBytecode = ID3D10Blob_GetBufferPointer(vsb);
        pd.VS.BytecodeLength = ID3D10Blob_GetBufferSize(vsb);
        pd.PS.pShaderBytecode = ID3D10Blob_GetBufferPointer(psb);
        pd.PS.BytecodeLength = ID3D10Blob_GetBufferSize(psb);
        pd.BlendState.RenderTarget[0].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
        pd.BlendState.RenderTarget[0].BlendEnable = TRUE;
        pd.BlendState.RenderTarget[0].SrcBlend = D3D11_BLEND_SRC_ALPHA;
        pd.BlendState.RenderTarget[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
        pd.BlendState.RenderTarget[0].BlendOp = D3D11_BLEND_OP_ADD;
        pd.BlendState.RenderTarget[0].SrcBlendAlpha = D3D11_BLEND_ONE;
        pd.BlendState.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_ZERO;
        pd.BlendState.RenderTarget[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
        pd.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID;
        pd.RasterizerState.CullMode = D3D12_CULL_MODE_NONE;
        pd.RasterizerState.FrontCounterClockwise = FALSE;
        pd.RasterizerState.DepthBias = D3D12_DEFAULT_DEPTH_BIAS;
        pd.RasterizerState.DepthBiasClamp = D3D12_DEFAULT_DEPTH_BIAS_CLAMP;
        pd.RasterizerState.SlopeScaledDepthBias = D3D12_DEFAULT_SLOPE_SCALED_DEPTH_BIAS;
        pd.RasterizerState.DepthClipEnable = TRUE;
        pd.RasterizerState.MultisampleEnable = FALSE;
        pd.RasterizerState.AntialiasedLineEnable = FALSE;
        pd.RasterizerState.ForcedSampleCount = 0;
        pd.RasterizerState.ConservativeRaster = D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF;
        pd.DepthStencilState.DepthEnable = FALSE;
        pd.DepthStencilState.StencilEnable = FALSE;
        pd.InputLayout.pInputElementDescs = layout;
        pd.InputLayout.NumElements = 2;
        pd.PrimitiveTopologyType = i ? D3D12_PRIMITIVE_TOPOLOGY_TYPE_LINE : D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
        pd.NumRenderTargets = 1;
        pd.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
        pd.SampleMask = 0xffffffffu;
        pd.SampleDesc.Count = 1;
        hr = ID3D12Device_CreateGraphicsPipelineState(iks_d12dev, &pd,
            &IID_ID3D12PipelineState, (void**)(i ? &iks_d12ps_line : &iks_d12ps_tri));
        if (FAILED(hr)) iks_d12_fail(hr, "CreateGraphicsPipelineState");
    }
    ID3D10Blob_Release(vsb);
    ID3D10Blob_Release(psb);
}

static void iks_d12_wait(void) {
    if (ID3D12Fence_GetCompletedValue(iks_d12fence) < iks_d12fval) {
        ID3D12Fence_SetEventOnCompletion(iks_d12fence, iks_d12fval, iks_d12fev);
        WaitForSingleObject(iks_d12fev, INFINITE);
    }
    iks_d12fval++;
}

static void iks_d12_init(int64_t w, int64_t h, const char* title) {
    if (iks_d12on) return;
    if (!iks_gfx_on) iks_gfx_open(w, h, title);
    HRESULT hr;
    /* factory */
    IDXGIFactory4* factory = NULL;
    hr = CreateDXGIFactory2(0, &IID_IDXGIFactory4, (void**)&factory);
    if (FAILED(hr)) iks_d12_fail(hr, "CreateDXGIFactory2");
    /* adapter (hardware, then WARP fallback) */
    IDXGIAdapter1* adapter = NULL;
    for (UINT i = 0; ; i++) {
        IDXGIAdapter1* a = NULL;
        if (IDXGIFactory1_EnumAdapters1((IDXGIFactory1*)factory, i, &a) == DXGI_ERROR_NOT_FOUND) break;
        /* try creating device on this adapter */
        ID3D12Device* tmp = NULL;
        if (SUCCEEDED(D3D12CreateDevice((IUnknown*)a, D3D_FEATURE_LEVEL_11_0, &IID_ID3D12Device, (void**)&tmp))) {
            ID3D12Device_Release(tmp);
            adapter = a;
            break;
        }
        IDXGIAdapter1_Release(a);
    }
    if (!adapter) {
        hr = IDXGIFactory4_EnumWarpAdapter(factory, &IID_IDXGIAdapter1, (void**)&adapter);
        if (FAILED(hr)) iks_d12_fail(hr, "EnumWarpAdapter");
    }
    /* device */
    hr = D3D12CreateDevice((IUnknown*)adapter, D3D_FEATURE_LEVEL_11_0, &IID_ID3D12Device, (void**)&iks_d12dev);
    if (FAILED(hr)) iks_d12_fail(hr, "D3D12CreateDevice");
    /* command queue */
    D3D12_COMMAND_QUEUE_DESC qd;
    ZeroMemory(&qd, sizeof(qd));
    qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    hr = ID3D12Device_CreateCommandQueue(iks_d12dev, &qd, &IID_ID3D12CommandQueue, (void**)&iks_d12q);
    if (FAILED(hr)) iks_d12_fail(hr, "CreateCommandQueue");
    /* swap chain (flip model) */
    DXGI_SWAP_CHAIN_DESC1 scd;
    ZeroMemory(&scd, sizeof(scd));
    scd.BufferCount = IKS_D12_FRAMES;
    scd.Width = iks_gw;
    scd.Height = iks_gh;
    scd.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    scd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    scd.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD;
    scd.SampleDesc.Count = 1;
    IDXGISwapChain1* sc1 = NULL;
    hr = IDXGIFactory2_CreateSwapChainForHwnd((IDXGIFactory2*)factory, (IUnknown*)iks_d12q, iks_hwnd, &scd, NULL, NULL, &sc1);
    if (FAILED(hr)) iks_d12_fail(hr, "CreateSwapChainForHwnd");
    hr = IDXGISwapChain1_QueryInterface(sc1, &IID_IDXGISwapChain3, (void**)&iks_d12sc);
    IDXGISwapChain1_Release(sc1);
    if (FAILED(hr)) iks_d12_fail(hr, "QueryInterface swapchain3");
    /* RTV heap */
    D3D12_DESCRIPTOR_HEAP_DESC hd;
    ZeroMemory(&hd, sizeof(hd));
    hd.NumDescriptors = IKS_D12_FRAMES;
    hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
    hr = ID3D12Device_CreateDescriptorHeap(iks_d12dev, &hd, &IID_ID3D12DescriptorHeap, (void**)&iks_d12rh);
    if (FAILED(hr)) iks_d12_fail(hr, "CreateDescriptorHeap RTV");
    iks_d12rtinc = ID3D12Device_GetDescriptorHandleIncrementSize(iks_d12dev, D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
    /* back buffers + RTVs */
    for (int i = 0; i < IKS_D12_FRAMES; i++) {
        hr = IDXGISwapChain3_GetBuffer(iks_d12sc, i, &IID_ID3D12Resource, (void**)&iks_d12rt[i]);
        if (FAILED(hr)) iks_d12_fail(hr, "GetBuffer");
        D3D12_CPU_DESCRIPTOR_HANDLE rtv = iks_d12_rtv((UINT)i);
        ID3D12Device_CreateRenderTargetView(iks_d12dev, iks_d12rt[i], NULL, rtv);
    }
    /* command allocator + list */
    for (int i = 0; i < IKS_D12_FRAMES; i++) {
        hr = ID3D12Device_CreateCommandAllocator(iks_d12dev, D3D12_COMMAND_LIST_TYPE_DIRECT,
            &IID_ID3D12CommandAllocator, (void**)&iks_d12ca[i]);
        if (FAILED(hr)) iks_d12_fail(hr, "CreateCommandAllocator");
    }
    hr = ID3D12Device_CreateCommandList(iks_d12dev, 0, D3D12_COMMAND_LIST_TYPE_DIRECT,
        iks_d12ca[0], NULL, &IID_ID3D12GraphicsCommandList, (void**)&iks_d12cl);
    if (FAILED(hr)) iks_d12_fail(hr, "CreateCommandList");
    /* close first recording */
    ID3D12GraphicsCommandList_Close(iks_d12cl);
    /* fence */
    hr = ID3D12Device_CreateFence(iks_d12dev, 0, D3D12_FENCE_FLAG_NONE,
        &IID_ID3D12Fence, (void**)&iks_d12fence);
    if (FAILED(hr)) iks_d12_fail(hr, "CreateFence");
    iks_d12fev = CreateEventA(NULL, FALSE, FALSE, NULL);
    iks_d12fval = 0;
    /* pipeline */
    iks_d12_make_pipeline();
    /* initial vertex buffer (64K host-visible) */
    iks_d12vbcap = 65536;
    D3D12_HEAP_PROPERTIES hp;
    ZeroMemory(&hp, sizeof(hp));
    hp.Type = D3D12_HEAP_TYPE_UPLOAD;
    D3D12_RESOURCE_DESC rd;
    ZeroMemory(&rd, sizeof(rd));
    rd.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    rd.Width = iks_d12vbcap;
    rd.Height = 1;
    rd.DepthOrArraySize = 1;
    rd.MipLevels = 1;
    rd.SampleDesc.Count = 1;
    rd.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    hr = ID3D12Device_CreateCommittedResource(iks_d12dev, &hp, D3D12_HEAP_FLAG_NONE,
        &rd, D3D12_RESOURCE_STATE_GENERIC_READ, NULL,
        &IID_ID3D12Resource, (void**)&iks_d12vb);
    if (FAILED(hr)) iks_d12_fail(hr, "CreateCommittedResource VB");
    hr = ID3D12Resource_Map(iks_d12vb, 0, NULL, &iks_d12vbm);
    if (FAILED(hr)) iks_d12_fail(hr, "Map VB");
    iks_d12fi = IDXGISwapChain3_GetCurrentBackBufferIndex(iks_d12sc);
    iks_d12on = 1;
    /* open first frame for recording */
    hr = ID3D12CommandAllocator_Reset(iks_d12ca[iks_d12fi]);
    if (FAILED(hr)) iks_d12_fail(hr, "CommandAllocator Reset init");
    hr = ID3D12GraphicsCommandList_Reset(iks_d12cl, iks_d12ca[iks_d12fi], NULL);
    if (FAILED(hr)) iks_d12_fail(hr, "CommandList Reset init");
    D3D12_CPU_DESCRIPTOR_HANDLE rtv0 = iks_d12_rtv(iks_d12fi);
    ID3D12GraphicsCommandList_OMSetRenderTargets(iks_d12cl, 1, &rtv0, FALSE, NULL);
    D3D12_VIEWPORT vp0;
    vp0.TopLeftX = 0.0f;
    vp0.TopLeftY = 0.0f;
    vp0.Width = (FLOAT)iks_gw;
    vp0.Height = (FLOAT)iks_gh;
    vp0.MinDepth = 0.0f;
    vp0.MaxDepth = 1.0f;
    ID3D12GraphicsCommandList_RSSetViewports(iks_d12cl, 1, &vp0);
    D3D12_RECT sr0 = { 0, 0, iks_gw, iks_gh };
    ID3D12GraphicsCommandList_RSSetScissorRects(iks_d12cl, 1, &sr0);
    ID3D12GraphicsCommandList_SetGraphicsRootSignature(iks_d12cl, iks_d12rs);
    iks_d12open = 1;
}

static void iks_d12_growbuf(UINT64 need) {
    UINT64 ncap = iks_d12vbcap;
    while (ncap < need) ncap *= 2;
    iks_d12_wait();
    ID3D12Resource_Unmap(iks_d12vb, 0, NULL);
    ID3D12Resource_Release(iks_d12vb);
    D3D12_HEAP_PROPERTIES hp;
    ZeroMemory(&hp, sizeof(hp));
    hp.Type = D3D12_HEAP_TYPE_UPLOAD;
    D3D12_RESOURCE_DESC rd;
    ZeroMemory(&rd, sizeof(rd));
    rd.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    rd.Width = ncap;
    rd.Height = 1;
    rd.DepthOrArraySize = 1;
    rd.MipLevels = 1;
    rd.SampleDesc.Count = 1;
    rd.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    HRESULT hr = ID3D12Device_CreateCommittedResource(iks_d12dev, &hp, D3D12_HEAP_FLAG_NONE,
        &rd, D3D12_RESOURCE_STATE_GENERIC_READ, NULL,
        &IID_ID3D12Resource, (void**)&iks_d12vb);
    if (FAILED(hr)) iks_d12_fail(hr, "CreateCommittedResource VB grow");
    hr = ID3D12Resource_Map(iks_d12vb, 0, NULL, &iks_d12vbm);
    if (FAILED(hr)) iks_d12_fail(hr, "Map VB grow");
    iks_d12vbcap = ncap;
}

static void iks_d12_color(int64_t r, int64_t g, int64_t b) {
    iks_d12cr = (float)iks_clamp8(r) / 255.0f;
    iks_d12cg = (float)iks_clamp8(g) / 255.0f;
    iks_d12cb = (float)iks_clamp8(b) / 255.0f;
}

static void iks_d12_begin(int64_t mode) {
    if (!iks_d12on) { fprintf(stderr, "d3d12 error: call dx12_init() first\n"); exit(1); }
    iks_d12mode = (mode == 1) ? 1 : 0;
    iks_d12in_begin = 1;
}

static void iks_d12_vertex(double x, double y, double z) {
    if (!iks_d12in_begin) return;
    if (iks_d12vn >= iks_d12vcap) {
        int nc = iks_d12vcap ? iks_d12vcap * 2 : 4096;
        IksD12V* nb = (IksD12V*)realloc(iks_d12vv, (size_t)nc * sizeof(IksD12V));
        if (!nb) { fprintf(stderr, "out of memory\n"); exit(1); }
        iks_d12vv = nb;
        iks_d12vcap = nc;
    }
    IksD12V* v = &iks_d12vv[iks_d12vn++];
    v->x = (float)x;
    v->y = (float)y;
    v->z = (float)z;
    v->r = iks_d12cr;
    v->g = iks_d12cg;
    v->b = iks_d12cb;
}

static void iks_d12_end(void) {
    if (!iks_d12in_begin) return;
    iks_d12in_begin = 0;
    if (iks_d12vn <= 0) return;
    UINT64 need = (UINT64)iks_d12vn * sizeof(IksD12V);
    if (need > iks_d12vbcap) iks_d12_growbuf(need);
    memcpy(iks_d12vbm, iks_d12vv, (size_t)need);
    /* set pipeline, IA, draw */
    ID3D12GraphicsCommandList_SetPipelineState(iks_d12cl,
        iks_d12mode ? iks_d12ps_line : iks_d12ps_tri);
    ID3D12GraphicsCommandList_IASetPrimitiveTopology(iks_d12cl,
        iks_d12mode ? D3D_PRIMITIVE_TOPOLOGY_LINELIST : D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    D3D12_VERTEX_BUFFER_VIEW vbv;
    vbv.BufferLocation = ID3D12Resource_GetGPUVirtualAddress(iks_d12vb);
    vbv.StrideInBytes = sizeof(IksD12V);
    vbv.SizeInBytes = (UINT)need;
    ID3D12GraphicsCommandList_IASetVertexBuffers(iks_d12cl, 0, 1, &vbv);
    ID3D12GraphicsCommandList_DrawInstanced(iks_d12cl, (UINT)iks_d12vn, 1, 0, 0);
    iks_d12vn = 0;
}

static void iks_d12_rect(double x, double y, double w, double h) {
    iks_d12_begin(3);
    iks_d12_vertex(x, y + h, 0.0);
    iks_d12_vertex(x + w, y, 0.0);
    iks_d12_vertex(x, y, 0.0);
    iks_d12_vertex(x, y + h, 0.0);
    iks_d12_vertex(x + w, y + h, 0.0);
    iks_d12_vertex(x + w, y, 0.0);
    iks_d12_end();
}

static void iks_d12_circle(double cx, double cy, double r, int64_t segs) {
    if (segs < 3) segs = 3;
    iks_d12_begin(3);
    for (int i = 0; i < segs; i++) {
        double a0 = (double)i / segs * 6.283185307179586;
        double a1 = (double)(i + 1) / segs * 6.283185307179586;
        iks_d12_vertex(cx, cy, 0.0);
        iks_d12_vertex(cx + r * cos(a0), cy + r * sin(a0), 0.0);
        iks_d12_vertex(cx + r * cos(a1), cy + r * sin(a1), 0.0);
    }
    iks_d12_end();
}

static void iks_d12_line(double x1, double y1, double x2, double y2) {
    iks_d12_begin(1);
    iks_d12_vertex(x1, y1, 0.0);
    iks_d12_vertex(x2, y2, 0.0);
    iks_d12_end();
}

static int64_t iks_d12_swap(void) {
    if (!iks_d12on) return 0;
    /* end frame: transition + present + fence */
    /* we've been recording into iks_d12cl with cmd alloc iks_d12ca[iks_d12fi] */
    HRESULT hr;
    ID3D12GraphicsCommandList_Close(iks_d12cl);
    ID3D12CommandQueue_ExecuteCommandLists(iks_d12q, 1, (ID3D12CommandList**)&iks_d12cl);
    hr = IDXGISwapChain3_Present(iks_d12sc, 0, 0);
    if (FAILED(hr) && hr != DXGI_ERROR_DEVICE_REMOVED && hr != DXGI_ERROR_DEVICE_RESET)
        iks_d12_fail(hr, "Present");
    /* signal fence for this frame */
    hr = ID3D12CommandQueue_Signal(iks_d12q, iks_d12fence, iks_d12fval);
    if (FAILED(hr)) iks_d12_fail(hr, "Signal");
    /* wait for this frame's fence before reusing its allocator next time */
    if (ID3D12Fence_GetCompletedValue(iks_d12fence) < iks_d12fval) {
        ID3D12Fence_SetEventOnCompletion(iks_d12fence, iks_d12fval, iks_d12fev);
        WaitForSingleObject(iks_d12fev, INFINITE);
    }
    iks_d12fval++;
    /* next frame */
    iks_d12fi = (iks_d12fi + 1) % IKS_D12_FRAMES;
    /* reset allocator + begin recording for new backbuffer */
    hr = ID3D12CommandAllocator_Reset(iks_d12ca[iks_d12fi]);
    if (FAILED(hr)) iks_d12_fail(hr, "CommandAllocator Reset");
    hr = ID3D12GraphicsCommandList_Reset(iks_d12cl, iks_d12ca[iks_d12fi], NULL);
    if (FAILED(hr)) iks_d12_fail(hr, "CommandList Reset");
    /* bind RTV, viewport, scissor, root sig */
    D3D12_CPU_DESCRIPTOR_HANDLE rtv = iks_d12_rtv(iks_d12fi);
    ID3D12GraphicsCommandList_OMSetRenderTargets(iks_d12cl, 1, &rtv, FALSE, NULL);
    D3D12_VIEWPORT vp;
    vp.TopLeftX = 0.0f;
    vp.TopLeftY = 0.0f;
    vp.Width = (FLOAT)iks_gw;
    vp.Height = (FLOAT)iks_gh;
    vp.MinDepth = 0.0f;
    vp.MaxDepth = 1.0f;
    ID3D12GraphicsCommandList_RSSetViewports(iks_d12cl, 1, &vp);
    D3D12_RECT sr = { 0, 0, iks_gw, iks_gh };
    ID3D12GraphicsCommandList_RSSetScissorRects(iks_d12cl, 1, &sr);
    ID3D12GraphicsCommandList_SetGraphicsRootSignature(iks_d12cl, iks_d12rs);
    iks_d12open = 1;
    return iks_gfx_poll();
}

static void iks_d12_clear(int64_t r, int64_t g, int64_t b) {
    if (!iks_d12on) { fprintf(stderr, "d3d12 error: call dx12_init() first\n"); exit(1); }
    /* already cleared at frame begin — no-op or reclear */
    D3D12_CPU_DESCRIPTOR_HANDLE rtv = iks_d12_rtv(iks_d12fi);
    float c[4];
    c[0] = (float)iks_clamp8(r) / 255.0f;
    c[1] = (float)iks_clamp8(g) / 255.0f;
    c[2] = (float)iks_clamp8(b) / 255.0f;
    c[3] = 1.0f;
    ID3D12GraphicsCommandList_ClearRenderTargetView(iks_d12cl, rtv, c, 0, NULL);
}

static void iks_d12_title(const char* t) {
    if (iks_hwnd) SetWindowTextA(iks_hwnd, t ? t : "IKS");
}

static int64_t iks_d12_width(void) { return iks_gw; }
static int64_t iks_d12_height(void) { return iks_gh; }

#if defined(IKS_VK) && IKS_VK
#define VK_USE_PLATFORM_WIN32_KHR
#include <vulkan/vulkan.h>

typedef struct { float x, y, z, r, g, b, a; } IksVkV;

static VkInstance iks_vki;
static VkSurfaceKHR iks_vksurf;
static VkPhysicalDevice iks_vkpd;
static VkDevice iks_vkdev;
static uint32_t iks_vkqf;
static VkQueue iks_vkq;
static VkSwapchainKHR iks_vksc;
static uint32_t iks_vkimgn;
static VkImage iks_vkimgs[8];
static VkImageView iks_vkviews[8];
static VkFramebuffer iks_vkfbs[8];
static VkFormat iks_vkfmt;
static VkExtent2D iks_vkext;
static VkRenderPass iks_vkrp;
static VkPipelineLayout iks_vkpl;
static VkPipeline iks_vkpipe[2]; /* 0=triangles 1=lines */
static VkCommandPool iks_vkcp;
static VkCommandBuffer iks_vkcb;
static VkSemaphore iks_vkimgsem, iks_vkrndsem;
static VkFence iks_vkfence;
static VkBuffer iks_vkbuf;
static VkDeviceMemory iks_vkmem;
static void* iks_vkmap;
static VkDeviceSize iks_vkbufsz, iks_vkused, iks_vkpeak;
static uint32_t iks_vkimg;
static int iks_vkrp_open;
static IksVkV* iks_vkv;
static int iks_vkvn, iks_vkvcap;
static int iks_vkmode, iks_vk_in_begin;
static float iks_vkcr, iks_vkg, iks_vkb;

static const uint32_t iks_vk_vert_spv[] = {0x07230203, 0x00010000, 0x0008000b, 0x00000034, 0x00000000, 0x00020011,
    0x00000001, 0x0006000b, 0x00000001, 0x4c534c47, 0x6474732e, 0x3035342e,
    0x00000000, 0x0003000e, 0x00000000, 0x00000001, 0x0009000f, 0x00000000,
    0x00000004, 0x6e69616d, 0x00000000, 0x0000000d, 0x00000012, 0x00000030,
    0x00000032, 0x00030003, 0x00000002, 0x000001c2, 0x00040005, 0x00000004,
    0x6e69616d, 0x00000000, 0x00060005, 0x0000000b, 0x505f6c67, 0x65567265,
    0x78657472, 0x00000000, 0x00060006, 0x0000000b, 0x00000000, 0x505f6c67,
    0x7469736f, 0x006e6f69, 0x00070006, 0x0000000b, 0x00000001, 0x505f6c67,
    0x746e696f, 0x657a6953, 0x00000000, 0x00070006, 0x0000000b, 0x00000002,
    0x435f6c67, 0x4470696c, 0x61747369, 0x0065636e, 0x00070006, 0x0000000b,
    0x00000003, 0x435f6c67, 0x446c6c75, 0x61747369, 0x0065636e, 0x00030005,
    0x0000000d, 0x00000000, 0x00040005, 0x00000012, 0x6f506e69, 0x00000073,
    0x00040005, 0x00000018, 0x68737550, 0x00000000, 0x00050006, 0x00000018,
    0x00000000, 0x65726373, 0x00006e65, 0x00030005, 0x0000001a, 0x00006370,
    0x00050005, 0x00000030, 0x67617266, 0x6f6c6f43, 0x00000072, 0x00040005,
    0x00000032, 0x6f436e69, 0x00726f6c, 0x00030047, 0x0000000b, 0x00000002,
    0x00050048, 0x0000000b, 0x00000000, 0x0000000b, 0x00000000, 0x00050048,
    0x0000000b, 0x00000001, 0x0000000b, 0x00000001, 0x00050048, 0x0000000b,
    0x00000002, 0x0000000b, 0x00000003, 0x00050048, 0x0000000b, 0x00000003,
    0x0000000b, 0x00000004, 0x00040047, 0x00000012, 0x0000001e, 0x00000000,
    0x00030047, 0x00000018, 0x00000002, 0x00050048, 0x00000018, 0x00000000,
    0x00000023, 0x00000000, 0x00040047, 0x00000030, 0x0000001e, 0x00000000,
    0x00040047, 0x00000032, 0x0000001e, 0x00000001, 0x00020013, 0x00000002,
    0x00030021, 0x00000003, 0x00000002, 0x00030016, 0x00000006, 0x00000020,
    0x00040017, 0x00000007, 0x00000006, 0x00000004, 0x00040015, 0x00000008,
    0x00000020, 0x00000000, 0x0004002b, 0x00000008, 0x00000009, 0x00000001,
    0x0004001c, 0x0000000a, 0x00000006, 0x00000009, 0x0006001e, 0x0000000b,
    0x00000007, 0x00000006, 0x0000000a, 0x0000000a, 0x00040020, 0x0000000c,
    0x00000003, 0x0000000b, 0x0004003b, 0x0000000c, 0x0000000d, 0x00000003,
    0x00040015, 0x0000000e, 0x00000020, 0x00000001, 0x0004002b, 0x0000000e,
    0x0000000f, 0x00000000, 0x00040017, 0x00000010, 0x00000006, 0x00000003,
    0x00040020, 0x00000011, 0x00000001, 0x00000010, 0x0004003b, 0x00000011,
    0x00000012, 0x00000001, 0x0004002b, 0x00000008, 0x00000013, 0x00000000,
    0x00040020, 0x00000014, 0x00000001, 0x00000006, 0x00040017, 0x00000017,
    0x00000006, 0x00000002, 0x0003001e, 0x00000018, 0x00000017, 0x00040020,
    0x00000019, 0x00000009, 0x00000018, 0x0004003b, 0x00000019, 0x0000001a,
    0x00000009, 0x00040020, 0x0000001b, 0x00000009, 0x00000006, 0x0004002b,
    0x00000006, 0x0000001f, 0x40000000, 0x0004002b, 0x00000006, 0x00000021,
    0x3f800000, 0x0004002b, 0x00000008, 0x0000002a, 0x00000002, 0x00040020,
    0x0000002e, 0x00000003, 0x00000007, 0x0004003b, 0x0000002e, 0x00000030,
    0x00000003, 0x00040020, 0x00000031, 0x00000001, 0x00000007, 0x0004003b,
    0x00000031, 0x00000032, 0x00000001, 0x00050036, 0x00000002, 0x00000004,
    0x00000000, 0x00000003, 0x000200f8, 0x00000005, 0x00050041, 0x00000014,
    0x00000015, 0x00000012, 0x00000013, 0x0004003d, 0x00000006, 0x00000016,
    0x00000015, 0x00060041, 0x0000001b, 0x0000001c, 0x0000001a, 0x0000000f,
    0x00000013, 0x0004003d, 0x00000006, 0x0000001d, 0x0000001c, 0x00050088,
    0x00000006, 0x0000001e, 0x00000016, 0x0000001d, 0x00050085, 0x00000006,
    0x00000020, 0x0000001e, 0x0000001f, 0x00050083, 0x00000006, 0x00000022,
    0x00000020, 0x00000021, 0x00050041, 0x00000014, 0x00000023, 0x00000012,
    0x00000009, 0x0004003d, 0x00000006, 0x00000024, 0x00000023, 0x00060041,
    0x0000001b, 0x00000025, 0x0000001a, 0x0000000f, 0x00000009, 0x0004003d,
    0x00000006, 0x00000026, 0x00000025, 0x00050088, 0x00000006, 0x00000027,
    0x00000024, 0x00000026, 0x00050085, 0x00000006, 0x00000028, 0x00000027,
    0x0000001f, 0x00050083, 0x00000006, 0x00000029, 0x00000028, 0x00000021,
    0x00050041, 0x00000014, 0x0000002b, 0x00000012, 0x0000002a, 0x0004003d,
    0x00000006, 0x0000002c, 0x0000002b, 0x00070050, 0x00000007, 0x0000002d,
    0x00000022, 0x00000029, 0x0000002c, 0x00000021, 0x00050041, 0x0000002e,
    0x0000002f, 0x0000000d, 0x0000000f, 0x0003003e, 0x0000002f, 0x0000002d,
    0x0004003d, 0x00000007, 0x00000033, 0x00000032, 0x0003003e, 0x00000030,
    0x00000033, 0x000100fd, 0x00010038 };

static const uint32_t iks_vk_frag_spv[] = {0x07230203, 0x00010000, 0x0008000b, 0x0000000d, 0x00000000, 0x00020011,
    0x00000001, 0x0006000b, 0x00000001, 0x4c534c47, 0x6474732e, 0x3035342e,
    0x00000000, 0x0003000e, 0x00000000, 0x00000001, 0x0007000f, 0x00000004,
    0x00000004, 0x6e69616d, 0x00000000, 0x00000009, 0x0000000b, 0x00030010,
    0x00000004, 0x00000007, 0x00030003, 0x00000002, 0x000001c2, 0x00040005,
    0x00000004, 0x6e69616d, 0x00000000, 0x00050005, 0x00000009, 0x4374756f,
    0x726f6c6f, 0x00000000, 0x00050005, 0x0000000b, 0x67617266, 0x6f6c6f43,
    0x00000072, 0x00040047, 0x00000009, 0x0000001e, 0x00000000, 0x00040047,
    0x0000000b, 0x0000001e, 0x00000000, 0x00020013, 0x00000002, 0x00030021,
    0x00000003, 0x00000002, 0x00030016, 0x00000006, 0x00000020, 0x00040017,
    0x00000007, 0x00000006, 0x00000004, 0x00040020, 0x00000008, 0x00000003,
    0x00000007, 0x0004003b, 0x00000008, 0x00000009, 0x00000003, 0x00040020,
    0x0000000a, 0x00000001, 0x00000007, 0x0004003b, 0x0000000a, 0x0000000b,
    0x00000001, 0x00050036, 0x00000002, 0x00000004, 0x00000000, 0x00000003,
    0x000200f8, 0x00000005, 0x0004003d, 0x00000007, 0x0000000c, 0x0000000b,
    0x0003003e, 0x00000009, 0x0000000c, 0x000100fd, 0x00010038 };

static void iks_vk_fail(VkResult r, const char* what) {
    fprintf(stderr, "vulkan error: %s (VkResult %d)\n", what, (int)r);
    exit(1);
}

static uint32_t iks_vk_find_mem(uint32_t typebits, VkMemoryPropertyFlags props) {
    VkPhysicalDeviceMemoryProperties mp;
    vkGetPhysicalDeviceMemoryProperties(iks_vkpd, &mp);
    for (uint32_t i = 0; i < mp.memoryTypeCount; i++) {
        if ((typebits & (1u << i)) && (mp.memoryTypes[i].propertyFlags & props) == props)
            return i;
    }
    fprintf(stderr, "vulkan error: no suitable memory type\n");
    exit(1);
    return 0;
}

static void iks_vk_makebuf(VkDeviceSize sz) {
    if (iks_vkmap) { vkUnmapMemory(iks_vkdev, iks_vkmem); iks_vkmap = NULL; }
    if (iks_vkbuf) vkDestroyBuffer(iks_vkdev, iks_vkbuf, NULL);
    if (iks_vkmem) vkFreeMemory(iks_vkdev, iks_vkmem, NULL);
    VkBufferCreateInfo bi; memset(&bi, 0, sizeof(bi));
    bi.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
    bi.size = sz;
    bi.usage = VK_BUFFER_USAGE_VERTEX_BUFFER_BIT;
    bi.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    VkResult r;
    if ((r = vkCreateBuffer(iks_vkdev, &bi, NULL, &iks_vkbuf)) != VK_SUCCESS) iks_vk_fail(r, "CreateBuffer");
    VkMemoryRequirements mr;
    vkGetBufferMemoryRequirements(iks_vkdev, iks_vkbuf, &mr);
    VkMemoryAllocateInfo ai; memset(&ai, 0, sizeof(ai));
    ai.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    ai.allocationSize = mr.size;
    ai.memoryTypeIndex = iks_vk_find_mem(mr.memoryTypeBits, VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT);
    if ((r = vkAllocateMemory(iks_vkdev, &ai, NULL, &iks_vkmem)) != VK_SUCCESS) iks_vk_fail(r, "AllocateMemory");
    if ((r = vkBindBufferMemory(iks_vkdev, iks_vkbuf, iks_vkmem, 0)) != VK_SUCCESS) iks_vk_fail(r, "BindBufferMemory");
    if ((r = vkMapMemory(iks_vkdev, iks_vkmem, 0, VK_WHOLE_SIZE, 0, &iks_vkmap)) != VK_SUCCESS) iks_vk_fail(r, "MapMemory");
    iks_vkbufsz = sz;
}

static void iks_vk_mkswapchain(void) {
    VkSurfaceCapabilitiesKHR caps;
    vkGetPhysicalDeviceSurfaceCapabilitiesKHR(iks_vkpd, iks_vksurf, &caps);
    uint32_t fc = 0;
    vkGetPhysicalDeviceSurfaceFormatsKHR(iks_vkpd, iks_vksurf, &fc, NULL);
    VkSurfaceFormatKHR fmts[32];
    if (fc > 32) fc = 32;
    vkGetPhysicalDeviceSurfaceFormatsKHR(iks_vkpd, iks_vksurf, &fc, fmts);
    VkSurfaceFormatKHR best = fmts[0];
    for (uint32_t i = 0; i < fc; i++) {
        if (fmts[i].format == VK_FORMAT_B8G8R8A8_UNORM || fmts[i].format == VK_FORMAT_R8G8B8A8_UNORM) { best = fmts[i]; break; }
    }
    iks_vkfmt = best.format;
    if (caps.currentExtent.width != 0xFFFFFFFF) iks_vkext = caps.currentExtent;
    else { iks_vkext.width = (uint32_t)iks_gw; iks_vkext.height = (uint32_t)iks_gh; }
    uint32_t n = caps.minImageCount + 1;
    if (caps.maxImageCount && n > caps.maxImageCount) n = caps.maxImageCount;
    VkSwapchainCreateInfoKHR si; memset(&si, 0, sizeof(si));
    si.sType = VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
    si.surface = iks_vksurf;
    si.minImageCount = n;
    si.imageFormat = best.format;
    si.imageColorSpace = best.colorSpace;
    si.imageExtent = iks_vkext;
    si.imageArrayLayers = 1;
    si.imageUsage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT;
    si.imageSharingMode = VK_SHARING_MODE_EXCLUSIVE;
    si.preTransform = caps.currentTransform;
    si.compositeAlpha = VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
    si.presentMode = VK_PRESENT_MODE_FIFO_KHR;
    si.clipped = VK_TRUE;
    VkResult r;
    if ((r = vkCreateSwapchainKHR(iks_vkdev, &si, NULL, &iks_vksc)) != VK_SUCCESS) iks_vk_fail(r, "CreateSwapchain");
    vkGetSwapchainImagesKHR(iks_vkdev, iks_vksc, &iks_vkimgn, NULL);
    if (iks_vkimgn > 8) iks_vkimgn = 8;
    vkGetSwapchainImagesKHR(iks_vkdev, iks_vksc, &iks_vkimgn, iks_vkimgs);
    for (uint32_t i = 0; i < iks_vkimgn; i++) {
        VkImageViewCreateInfo vi; memset(&vi, 0, sizeof(vi));
        vi.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
        vi.image = iks_vkimgs[i];
        vi.viewType = VK_IMAGE_VIEW_TYPE_2D;
        vi.format = iks_vkfmt;
        vi.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
        vi.subresourceRange.levelCount = 1;
        vi.subresourceRange.layerCount = 1;
        if ((r = vkCreateImageView(iks_vkdev, &vi, NULL, &iks_vkviews[i])) != VK_SUCCESS) iks_vk_fail(r, "CreateImageView");
        VkFramebufferCreateInfo fci; memset(&fci, 0, sizeof(fci));
        fci.sType = VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO;
        fci.renderPass = iks_vkrp;
        fci.attachmentCount = 1;
        fci.pAttachments = &iks_vkviews[i];
        fci.width = iks_vkext.width;
        fci.height = iks_vkext.height;
        fci.layers = 1;
        if ((r = vkCreateFramebuffer(iks_vkdev, &fci, NULL, &iks_vkfbs[i])) != VK_SUCCESS) iks_vk_fail(r, "CreateFramebuffer");
    }
}

static void iks_vk_recreate(void) {
    vkDeviceWaitIdle(iks_vkdev);
    for (uint32_t i = 0; i < iks_vkimgn; i++) {
        vkDestroyFramebuffer(iks_vkdev, iks_vkfbs[i], NULL);
        vkDestroyImageView(iks_vkdev, iks_vkviews[i], NULL);
    }
    vkDestroySwapchainKHR(iks_vkdev, iks_vksc, NULL);
    iks_vk_mkswapchain();
}

static void iks_vk_endframe(void) {
    if (!iks_vkrp_open) return;
    vkCmdEndRenderPass(iks_vkcb);
    VkResult r;
    if ((r = vkEndCommandBuffer(iks_vkcb)) != VK_SUCCESS) iks_vk_fail(r, "EndCommandBuffer");
    VkPipelineStageFlags wmask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
    VkSubmitInfo si; memset(&si, 0, sizeof(si));
    si.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    si.waitSemaphoreCount = 1;
    si.pWaitSemaphores = &iks_vkimgsem;
    si.pWaitDstStageMask = &wmask;
    si.commandBufferCount = 1;
    si.pCommandBuffers = &iks_vkcb;
    si.signalSemaphoreCount = 1;
    si.pSignalSemaphores = &iks_vkrndsem;
    if ((r = vkQueueSubmit(iks_vkq, 1, &si, iks_vkfence)) != VK_SUCCESS) iks_vk_fail(r, "QueueSubmit");
    VkPresentInfoKHR pi; memset(&pi, 0, sizeof(pi));
    pi.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
    pi.waitSemaphoreCount = 1;
    pi.pWaitSemaphores = &iks_vkrndsem;
    pi.swapchainCount = 1;
    pi.pSwapchains = &iks_vksc;
    pi.pImageIndices = &iks_vkimg;
    r = vkQueuePresentKHR(iks_vkq, &pi);
    if (r == VK_ERROR_OUT_OF_DATE_KHR || r == VK_SUBOPTIMAL_KHR) {
        iks_vk_recreate();
    } else if (r != VK_SUCCESS) {
        iks_vk_fail(r, "QueuePresent");
    }
    vkQueueWaitIdle(iks_vkq);
    iks_vkrp_open = 0;
    if (iks_vkpeak * 2 > iks_vkbufsz) {
        iks_vk_makebuf(iks_vkbufsz * 2);
        iks_vkpeak = 0;
    }
}

static void iks_vk_init(int64_t w, int64_t h, const char* title) {
    if (iks_vkon) return;
    if (!iks_gfx_on) iks_gfx_open(w, h, title);
    VkResult r;
    VkApplicationInfo ai; memset(&ai, 0, sizeof(ai));
    ai.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    ai.pApplicationName = "IKS";
    ai.apiVersion = VK_API_VERSION_1_0;
    const char* exts[2] = { VK_KHR_SURFACE_EXTENSION_NAME, VK_KHR_WIN32_SURFACE_EXTENSION_NAME };
    VkInstanceCreateInfo ici; memset(&ici, 0, sizeof(ici));
    ici.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    ici.pApplicationInfo = &ai;
    ici.enabledExtensionCount = 2;
    ici.ppEnabledExtensionNames = exts;
    if ((r = vkCreateInstance(&ici, NULL, &iks_vki)) != VK_SUCCESS) iks_vk_fail(r, "CreateInstance");
    VkWin32SurfaceCreateInfoKHR wsi; memset(&wsi, 0, sizeof(wsi));
    wsi.sType = VK_STRUCTURE_TYPE_WIN32_SURFACE_CREATE_INFO_KHR;
    wsi.hinstance = GetModuleHandleA(NULL);
    wsi.hwnd = iks_hwnd;
    if ((r = vkCreateWin32SurfaceKHR(iks_vki, &wsi, NULL, &iks_vksurf)) != VK_SUCCESS) iks_vk_fail(r, "CreateWin32Surface");
    uint32_t pdn = 0;
    vkEnumeratePhysicalDevices(iks_vki, &pdn, NULL);
    if (!pdn) { fprintf(stderr, "vulkan error: no physical devices\n"); exit(1); }
    VkPhysicalDevice* pds = (VkPhysicalDevice*)malloc(pdn * sizeof(VkPhysicalDevice));
    vkEnumeratePhysicalDevices(iks_vki, &pdn, pds);
    int found = 0;
    for (uint32_t i = 0; i < pdn && !found; i++) {
        uint32_t qn = 0;
        vkGetPhysicalDeviceQueueFamilyProperties(pds[i], &qn, NULL);
        VkQueueFamilyProperties* qps = (VkQueueFamilyProperties*)malloc(qn * sizeof(VkQueueFamilyProperties));
        vkGetPhysicalDeviceQueueFamilyProperties(pds[i], &qn, qps);
        for (uint32_t q = 0; q < qn; q++) {
            VkBool32 pres = VK_FALSE;
            vkGetPhysicalDeviceSurfaceSupportKHR(pds[i], q, iks_vksurf, &pres);
            if ((qps[q].queueFlags & VK_QUEUE_GRAPHICS_BIT) && pres) {
                iks_vkpd = pds[i];
                iks_vkqf = q;
                found = 1;
                break;
            }
        }
        free(qps);
    }
    free(pds);
    if (!found) { fprintf(stderr, "vulkan error: no suitable GPU\n"); exit(1); }
    float prio = 1.0f;
    VkDeviceQueueCreateInfo qci; memset(&qci, 0, sizeof(qci));
    qci.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
    qci.queueFamilyIndex = iks_vkqf;
    qci.queueCount = 1;
    qci.pQueuePriorities = &prio;
    const char* dexts[1] = { VK_KHR_SWAPCHAIN_EXTENSION_NAME };
    VkDeviceCreateInfo dci; memset(&dci, 0, sizeof(dci));
    dci.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
    dci.queueCreateInfoCount = 1;
    dci.pQueueCreateInfos = &qci;
    dci.enabledExtensionCount = 1;
    dci.ppEnabledExtensionNames = dexts;
    if ((r = vkCreateDevice(iks_vkpd, &dci, NULL, &iks_vkdev)) != VK_SUCCESS) iks_vk_fail(r, "CreateDevice");
    vkGetDeviceQueue(iks_vkdev, iks_vkqf, 0, &iks_vkq);
    uint32_t fc = 0;
    vkGetPhysicalDeviceSurfaceFormatsKHR(iks_vkpd, iks_vksurf, &fc, NULL);
    VkSurfaceFormatKHR fmts[32];
    if (fc > 32) fc = 32;
    vkGetPhysicalDeviceSurfaceFormatsKHR(iks_vkpd, iks_vksurf, &fc, fmts);
    VkSurfaceFormatKHR best = fmts[0];
    for (uint32_t i = 0; i < fc; i++)
        if (fmts[i].format == VK_FORMAT_B8G8R8A8_UNORM || fmts[i].format == VK_FORMAT_R8G8B8A8_UNORM) { best = fmts[i]; break; }
    iks_vkfmt = best.format;
    VkAttachmentDescription ad; memset(&ad, 0, sizeof(ad));
    ad.format = iks_vkfmt;
    ad.samples = VK_SAMPLE_COUNT_1_BIT;
    ad.loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR;
    ad.storeOp = VK_ATTACHMENT_STORE_OP_STORE;
    ad.stencilLoadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;
    ad.stencilStoreOp = VK_ATTACHMENT_STORE_OP_DONT_CARE;
    ad.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    ad.finalLayout = VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;
    VkAttachmentReference aref; memset(&aref, 0, sizeof(aref));
    aref.attachment = 0;
    aref.layout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;
    VkSubpassDescription sd; memset(&sd, 0, sizeof(sd));
    sd.pipelineBindPoint = VK_PIPELINE_BIND_POINT_GRAPHICS;
    sd.colorAttachmentCount = 1;
    sd.pColorAttachments = &aref;
    VkSubpassDependency dep; memset(&dep, 0, sizeof(dep));
    dep.srcSubpass = VK_SUBPASS_EXTERNAL;
    dep.dstSubpass = 0;
    dep.srcStageMask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
    dep.dstStageMask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
    dep.dstAccessMask = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
    VkRenderPassCreateInfo rpi; memset(&rpi, 0, sizeof(rpi));
    rpi.sType = VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO;
    rpi.attachmentCount = 1;
    rpi.pAttachments = &ad;
    rpi.subpassCount = 1;
    rpi.pSubpasses = &sd;
    rpi.dependencyCount = 1;
    rpi.pDependencies = &dep;
    if ((r = vkCreateRenderPass(iks_vkdev, &rpi, NULL, &iks_vkrp)) != VK_SUCCESS) iks_vk_fail(r, "CreateRenderPass");
    iks_vk_mkswapchain();
    VkShaderModule vsm, fsm;
    VkShaderModuleCreateInfo smi; memset(&smi, 0, sizeof(smi));
    smi.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
    smi.codeSize = sizeof(iks_vk_vert_spv);
    smi.pCode = iks_vk_vert_spv;
    if ((r = vkCreateShaderModule(iks_vkdev, &smi, NULL, &vsm)) != VK_SUCCESS) iks_vk_fail(r, "CreateShaderModule vert");
    smi.codeSize = sizeof(iks_vk_frag_spv);
    smi.pCode = iks_vk_frag_spv;
    if ((r = vkCreateShaderModule(iks_vkdev, &smi, NULL, &fsm)) != VK_SUCCESS) iks_vk_fail(r, "CreateShaderModule frag");
    VkPushConstantRange pcr; memset(&pcr, 0, sizeof(pcr));
    pcr.stageFlags = VK_SHADER_STAGE_VERTEX_BIT;
    pcr.size = 8;
    VkPipelineLayoutCreateInfo pli; memset(&pli, 0, sizeof(pli));
    pli.sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
    pli.pushConstantRangeCount = 1;
    pli.pPushConstantRanges = &pcr;
    if ((r = vkCreatePipelineLayout(iks_vkdev, &pli, NULL, &iks_vkpl)) != VK_SUCCESS) iks_vk_fail(r, "CreatePipelineLayout");
    for (int pi = 0; pi < 2; pi++) {
        VkPipelineShaderStageCreateInfo ss[2]; memset(ss, 0, sizeof(ss));
        ss[0].sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
        ss[0].stage = VK_SHADER_STAGE_VERTEX_BIT;
        ss[0].module = vsm;
        ss[0].pName = "main";
        ss[1].sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
        ss[1].stage = VK_SHADER_STAGE_FRAGMENT_BIT;
        ss[1].module = fsm;
        ss[1].pName = "main";
        VkVertexInputBindingDescription vb; memset(&vb, 0, sizeof(vb));
        vb.binding = 0;
        vb.stride = sizeof(IksVkV);
        vb.inputRate = VK_VERTEX_INPUT_RATE_VERTEX;
        VkVertexInputAttributeDescription va[2]; memset(va, 0, sizeof(va));
        va[0].location = 0;
        va[0].format = VK_FORMAT_R32G32B32_SFLOAT;
        va[0].offset = 0;
        va[1].location = 1;
        va[1].format = VK_FORMAT_R32G32B32A32_SFLOAT;
        va[1].offset = 12;
        VkPipelineVertexInputStateCreateInfo vi; memset(&vi, 0, sizeof(vi));
        vi.sType = VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO;
        vi.vertexBindingDescriptionCount = 1;
        vi.pVertexBindingDescriptions = &vb;
        vi.vertexAttributeDescriptionCount = 2;
        vi.pVertexAttributeDescriptions = va;
        VkPipelineInputAssemblyStateCreateInfo ia; memset(&ia, 0, sizeof(ia));
        ia.sType = VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO;
        ia.topology = pi ? VK_PRIMITIVE_TOPOLOGY_LINE_LIST : VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST;
        VkPipelineViewportStateCreateInfo vs; memset(&vs, 0, sizeof(vs));
        vs.sType = VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO;
        vs.viewportCount = 1;
        vs.scissorCount = 1;
        VkPipelineRasterizationStateCreateInfo rs; memset(&rs, 0, sizeof(rs));
        rs.sType = VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO;
        rs.polygonMode = VK_POLYGON_MODE_FILL;
        rs.cullMode = VK_CULL_MODE_NONE;
        rs.frontFace = VK_FRONT_FACE_COUNTER_CLOCKWISE;
        rs.lineWidth = 1.0f;
        VkPipelineMultisampleStateCreateInfo ms; memset(&ms, 0, sizeof(ms));
        ms.sType = VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO;
        ms.rasterizationSamples = VK_SAMPLE_COUNT_1_BIT;
        VkPipelineColorBlendAttachmentState cba; memset(&cba, 0, sizeof(cba));
        cba.colorWriteMask = 0xF;
        cba.blendEnable = VK_TRUE;
        cba.srcColorBlendFactor = VK_BLEND_FACTOR_SRC_ALPHA;
        cba.dstColorBlendFactor = VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA;
        cba.colorBlendOp = VK_BLEND_OP_ADD;
        cba.srcAlphaBlendFactor = VK_BLEND_FACTOR_ONE;
        cba.dstAlphaBlendFactor = VK_BLEND_FACTOR_ZERO;
        cba.alphaBlendOp = VK_BLEND_OP_ADD;
        VkPipelineColorBlendStateCreateInfo cb; memset(&cb, 0, sizeof(cb));
        cb.sType = VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO;
        cb.attachmentCount = 1;
        cb.pAttachments = &cba;
        VkDynamicState dyn[2] = { VK_DYNAMIC_STATE_VIEWPORT, VK_DYNAMIC_STATE_SCISSOR };
        VkPipelineDynamicStateCreateInfo ds; memset(&ds, 0, sizeof(ds));
        ds.sType = VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO;
        ds.dynamicStateCount = 2;
        ds.pDynamicStates = dyn;
        VkGraphicsPipelineCreateInfo gpi; memset(&gpi, 0, sizeof(gpi));
        gpi.sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
        gpi.stageCount = 2;
        gpi.pStages = ss;
        gpi.pVertexInputState = &vi;
        gpi.pInputAssemblyState = &ia;
        gpi.pViewportState = &vs;
        gpi.pRasterizationState = &rs;
        gpi.pMultisampleState = &ms;
        gpi.pColorBlendState = &cb;
        gpi.pDynamicState = &ds;
        gpi.layout = iks_vkpl;
        gpi.renderPass = iks_vkrp;
        gpi.subpass = 0;
        if ((r = vkCreateGraphicsPipelines(iks_vkdev, VK_NULL_HANDLE, 1, &gpi, NULL, &iks_vkpipe[pi])) != VK_SUCCESS) iks_vk_fail(r, "CreateGraphicsPipelines");
    }
    vkDestroyShaderModule(iks_vkdev, vsm, NULL);
    vkDestroyShaderModule(iks_vkdev, fsm, NULL);
    VkCommandPoolCreateInfo cpi; memset(&cpi, 0, sizeof(cpi));
    cpi.sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
    cpi.flags = VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT;
    cpi.queueFamilyIndex = iks_vkqf;
    if ((r = vkCreateCommandPool(iks_vkdev, &cpi, NULL, &iks_vkcp)) != VK_SUCCESS) iks_vk_fail(r, "CreateCommandPool");
    VkCommandBufferAllocateInfo cbai; memset(&cbai, 0, sizeof(cbai));
    cbai.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
    cbai.commandPool = iks_vkcp;
    cbai.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
    cbai.commandBufferCount = 1;
    if ((r = vkAllocateCommandBuffers(iks_vkdev, &cbai, &iks_vkcb)) != VK_SUCCESS) iks_vk_fail(r, "AllocateCommandBuffers");
    VkSemaphoreCreateInfo sci; memset(&sci, 0, sizeof(sci));
    sci.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
    vkCreateSemaphore(iks_vkdev, &sci, NULL, &iks_vkimgsem);
    vkCreateSemaphore(iks_vkdev, &sci, NULL, &iks_vkrndsem);
    VkFenceCreateInfo fci; memset(&fci, 0, sizeof(fci));
    fci.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
    fci.flags = VK_FENCE_CREATE_SIGNALED_BIT;
    vkCreateFence(iks_vkdev, &fci, NULL, &iks_vkfence);
    iks_vk_makebuf(4 << 20);
    iks_vkon = 1;
}

static void iks_vk_clear(int64_t rr, int64_t gg, int64_t bb) {
    if (!iks_vkon) { fprintf(stderr, "vulkan error: call vk_init() first\n"); exit(1); }
    iks_vk_endframe();
    VkResult r;
    if ((r = vkWaitForFences(iks_vkdev, 1, &iks_vkfence, VK_TRUE, UINT64_MAX)) != VK_SUCCESS) iks_vk_fail(r, "WaitFences");
    for (;;) {
        r = vkAcquireNextImageKHR(iks_vkdev, iks_vksc, UINT64_MAX, iks_vkimgsem, VK_NULL_HANDLE, &iks_vkimg);
        if (r == VK_ERROR_OUT_OF_DATE_KHR) { iks_vk_recreate(); continue; }
        if (r != VK_SUCCESS && r != VK_SUBOPTIMAL_KHR) iks_vk_fail(r, "AcquireNextImage");
        break;
    }
    if ((r = vkResetFences(iks_vkdev, 1, &iks_vkfence)) != VK_SUCCESS) iks_vk_fail(r, "ResetFences");
    if ((r = vkResetCommandBuffer(iks_vkcb, 0)) != VK_SUCCESS) iks_vk_fail(r, "ResetCommandBuffer");
    VkCommandBufferBeginInfo bi; memset(&bi, 0, sizeof(bi));
    bi.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    bi.flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT;
    if ((r = vkBeginCommandBuffer(iks_vkcb, &bi)) != VK_SUCCESS) iks_vk_fail(r, "BeginCommandBuffer");
    VkClearValue cv; memset(&cv, 0, sizeof(cv));
    cv.color.float32[0] = (float)iks_clamp8(rr) / 255.0f;
    cv.color.float32[1] = (float)iks_clamp8(gg) / 255.0f;
    cv.color.float32[2] = (float)iks_clamp8(bb) / 255.0f;
    cv.color.float32[3] = 1.0f;
    VkRenderPassBeginInfo rp; memset(&rp, 0, sizeof(rp));
    rp.sType = VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO;
    rp.renderPass = iks_vkrp;
    rp.framebuffer = iks_vkfbs[iks_vkimg];
    rp.renderArea.extent = iks_vkext;
    rp.clearValueCount = 1;
    rp.pClearValues = &cv;
    vkCmdBeginRenderPass(iks_vkcb, &rp, VK_SUBPASS_CONTENTS_INLINE);
    VkViewport vp; memset(&vp, 0, sizeof(vp));
    vp.width = (float)iks_vkext.width;
    vp.height = (float)iks_vkext.height;
    vp.maxDepth = 1.0f;
    vkCmdSetViewport(iks_vkcb, 0, 1, &vp);
    VkRect2D sc; memset(&sc, 0, sizeof(sc));
    sc.extent = iks_vkext;
    vkCmdSetScissor(iks_vkcb, 0, 1, &sc);
    float screen[2] = { (float)iks_vkext.width, (float)iks_vkext.height };
    vkCmdPushConstants(iks_vkcb, iks_vkpl, VK_SHADER_STAGE_VERTEX_BIT, 0, 8, screen);
    iks_vkrp_open = 1;
    iks_vkvn = 0;
}

static void iks_vk_color(int64_t r, int64_t g, int64_t b) {
    iks_vkcr = (float)iks_clamp8(r) / 255.0f;
    iks_vkg = (float)iks_clamp8(g) / 255.0f;
    iks_vkb = (float)iks_clamp8(b) / 255.0f;
}

static void iks_vk_begin(int64_t mode) {
    if (!iks_vkrp_open) { fprintf(stderr, "vulkan error: call vk_clear() first\n"); exit(1); }
    iks_vkmode = (mode == 1) ? 1 : 0;
    iks_vk_in_begin = 1;
}

static void iks_vk_vertex(double x, double y, double z) {
    if (!iks_vk_in_begin) return;
    if (iks_vkvn >= iks_vkvcap) {
        int nc = iks_vkvcap ? iks_vkvcap * 2 : 4096;
        IksVkV* nb = (IksVkV*)realloc(iks_vkv, (size_t)nc * sizeof(IksVkV));
        if (!nb) { fprintf(stderr, "out of memory\n"); exit(1); }
        iks_vkv = nb;
        iks_vkvcap = nc;
    }
    IksVkV* v = &iks_vkv[iks_vkvn++];
    v->x = (float)x;
    v->y = (float)y;
    v->z = (float)z;
    v->r = iks_vkcr;
    v->g = iks_vkg;
    v->b = iks_vkb;
    v->a = 1.0f;
}

static void iks_vk_end(void) {
    if (!iks_vk_in_begin) return;
    iks_vk_in_begin = 0;
    if (iks_vkvn <= 0) return;
    VkDeviceSize need = (VkDeviceSize)iks_vkvn * sizeof(IksVkV);
    if (iks_vkused + need > iks_vkbufsz) {
        fprintf(stderr, "vulkan error: vertex buffer full (%d verts this frame); split drawing across frames\n", iks_vkvn);
        iks_vkvn = 0;
        return;
    }
    memcpy((char*)iks_vkmap + iks_vkused, iks_vkv, (size_t)need);
    VkDeviceSize off = iks_vkused;
    uint32_t count = (uint32_t)iks_vkvn;
    vkCmdBindPipeline(iks_vkcb, VK_PIPELINE_BIND_POINT_GRAPHICS, iks_vkpipe[iks_vkmode]);
    vkCmdBindVertexBuffers(iks_vkcb, 0, 1, &iks_vkbuf, &off);
    vkCmdDraw(iks_vkcb, count, 1, 0, 0);
    iks_vkused += need;
    if (iks_vkused > iks_vkpeak) iks_vkpeak = iks_vkused;
    iks_vkvn = 0;
}

static void iks_vk_rect(double x, double y, double w, double h) {
    float cr = iks_vkcr, cg = iks_vkg, cb = iks_vkb;
    iks_vk_begin(3);
    struct { float x, y; } q[6] = {
        { (float)x, (float)(y + h) }, { (float)(x + w), (float)y }, { (float)x, (float)y },
        { (float)x, (float)(y + h) }, { (float)(x + w), (float)(y + h) }, { (float)(x + w), (float)y }
    };
    for (int i = 0; i < 6; i++) iks_vk_vertex(q[i].x, q[i].y, 0.0);
    iks_vk_end();
    iks_vkcr = cr; iks_vkg = cg; iks_vkb = cb;
}

static void iks_vk_circle(double cx, double cy, double r, int64_t segs) {
    if (segs < 3) segs = 3;
    iks_vk_begin(3);
    for (int i = 0; i < segs; i++) {
        double a0 = (double)i / segs * 6.283185307179586;
        double a1 = (double)(i + 1) / segs * 6.283185307179586;
        iks_vk_vertex(cx, cy, 0.0);
        iks_vk_vertex(cx + r * cos(a0), cy + r * sin(a0), 0.0);
        iks_vk_vertex(cx + r * cos(a1), cy + r * sin(a1), 0.0);
    }
    iks_vk_end();
}

static void iks_vk_line(double x1, double y1, double x2, double y2) {
    iks_vk_begin(1);
    iks_vk_vertex(x1, y1, 0.0);
    iks_vk_vertex(x2, y2, 0.0);
    iks_vk_end();
}

static int64_t iks_vk_swap(void) {
    if (!iks_vkon) return 0;
    iks_vk_endframe();
    return iks_gfx_poll();
}

static void iks_vk_title(const char* t) {
    if (iks_hwnd) SetWindowTextA(iks_hwnd, t ? t : "IKS");
}

static int64_t iks_vk_width(void) { return iks_gw; }
static int64_t iks_vk_height(void) { return iks_gh; }

#else /* IKS_VK not defined */

static void iks_vk_init(int64_t w, int64_t h, const char* title) {
    (void)w; (void)h; (void)title;
    fprintf(stderr, "vulkan error: not built with Vulkan support (Vulkan SDK not found at compile time)\n");
    exit(1);
}
static void iks_vk_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_vk_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_vk_begin(int64_t m) { (void)m; }
static void iks_vk_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_vk_end(void) {}
static void iks_vk_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_vk_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_vk_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_vk_swap(void) { return 0; }
static void iks_vk_title(const char* t) { (void)t; }
static int64_t iks_vk_width(void) { return 0; }
static int64_t iks_vk_height(void) { return 0; }

#endif /* IKS_VK */

#elif defined(IKS_ANDROID)

#include <android/log.h>
#include <android/native_window.h>
#include <android/input.h>
#include <android/looper.h>
#include <android_native_app_glue.h>
#include <EGL/egl.h>
#include <GLES2/gl2.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>
#include <unistd.h>

#define IKALOG(...) __android_log_print(ANDROID_LOG_INFO, "IKS", __VA_ARGS__)
#define IKERR(...)  __android_log_print(ANDROID_LOG_ERROR, "IKS", __VA_ARGS__)

int main(void);
static int64_t iks_gfx_poll(void);

/* ---- shared state (Android equivalents of the Windows globals) ---- */
static void* iks_pixels = NULL;
static int iks_gw = 0, iks_gh = 0;
static int iks_gfx_on = 0, iks_gfx_closed = 0;
static double iks_gfx_t0 = 0.0;
static unsigned char iks_keys[256];
static int iks_mx = 0, iks_my = 0;
static unsigned char iks_mbtn[3];

static double iks_now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec / 1e9;
}

static unsigned char iks_clamp8(int64_t v) {
    if (v < 0) return 0;
    if (v > 255) return 255;
    return (unsigned char)v;
}

/* ---- EGL / app globals ---- */
static struct android_app* iks_app = NULL;
static EGLDisplay iks_edpy = EGL_NO_DISPLAY;
static EGLSurface iks_esurf = EGL_NO_SURFACE;
static EGLContext iks_ectx = EGL_NO_CONTEXT;
static EGLConfig iks_ecfg;
static int iks_egl_inited = 0;
static int iks_win_w = 0, iks_win_h = 0;

/* ---- column-major mat4 math ---- */
typedef struct { float m[16]; } IksM;
static IksM iks_ident(void) {
    IksM r;
    memset(r.m, 0, sizeof r.m);
    r.m[0] = r.m[5] = r.m[10] = r.m[15] = 1.0f;
    return r;
}
static IksM iks_mmul(IksM a, IksM b) {
    IksM r;
    for (int c = 0; c < 4; c++)
        for (int rw = 0; rw < 4; rw++) {
            float s = 0.0f;
            for (int k = 0; k < 4; k++) s += a.m[k*4+rw] * b.m[c*4+k];
            r.m[c*4+rw] = s;
        }
    return r;
}
static IksM iks_mfrustum(double l, double r, double b, double t, double n, double f) {
    IksM m;
    memset(m.m, 0, sizeof m.m);
    m.m[0] = (float)(2.0*n/(r-l));
    m.m[5] = (float)(2.0*n/(t-b));
    m.m[8] = (float)((r+l)/(r-l));
    m.m[9] = (float)((t+b)/(t-b));
    m.m[10] = (float)(-(f+n)/(f-n));
    m.m[11] = -1.0f;
    m.m[14] = (float)(-2.0*f*n/(f-n));
    return m;
}
static IksM iks_mortho(double l, double r, double b, double t, double n, double f) {
    IksM m;
    memset(m.m, 0, sizeof m.m);
    m.m[0] = (float)(2.0/(r-l));
    m.m[5] = (float)(2.0/(t-b));
    m.m[10] = (float)(-2.0/(f-n));
    m.m[12] = (float)(-(r+l)/(r-l));
    m.m[13] = (float)(-(t+b)/(t-b));
    m.m[14] = (float)(-(f+n)/(f-n));
    m.m[15] = 1.0f;
    return m;
}
static IksM iks_mtrans(double x, double y, double z) {
    IksM m = iks_ident();
    m.m[12] = (float)x; m.m[13] = (float)y; m.m[14] = (float)z;
    return m;
}
static IksM iks_mscale(double x, double y, double z) {
    IksM m = iks_ident();
    m.m[0] = (float)x; m.m[5] = (float)y; m.m[10] = (float)z;
    return m;
}
static IksM iks_mrot(double ang, double x, double y, double z) {
    double len = sqrt(x*x + y*y + z*z);
    if (len <= 1e-9) return iks_ident();
    x /= len; y /= len; z /= len;
    double a = ang * 3.14159265358979323846 / 180.0;
    double c = cos(a), s = sin(a), t = 1.0 - c;
    IksM m = iks_ident();
    m.m[0] = (float)(t*x*x + c);
    m.m[1] = (float)(t*x*y + s*z);
    m.m[2] = (float)(t*x*z - s*y);
    m.m[4] = (float)(t*x*y - s*z);
    m.m[5] = (float)(t*y*y + c);
    m.m[6] = (float)(t*y*z + s*x);
    m.m[8] = (float)(t*x*z + s*y);
    m.m[9] = (float)(t*y*z - s*x);
    m.m[10] = (float)(t*z*z + c);
    return m;
}

static IksM iks_pmat, iks_mmat;
static IksM iks_mstack[64];
static int iks_mtop = 0;

/* ---- GLES2 program state ---- */
static GLuint iks_prog = 0;
static GLint iks_a_pos = -1, iks_a_col = -1, iks_a_uv = -1;
static GLint iks_u_mvp = -1, iks_u_psize = -1, iks_u_tex = -1;
static GLuint iks_white_tex = 0;
static GLuint iks_fb_tex = 0;
static int iks_glstate_ok = 0;
static float iks_psize = 1.0f;
static GLuint iks_cur_tex = 0; /* 0 => white */

static const char* IKS_VSH =
    "#version 100\n"
    "attribute vec3 a_pos;\n"
    "attribute vec4 a_col;\n"
    "attribute vec2 a_uv;\n"
    "uniform mat4 u_mvp;\n"
    "uniform float u_psize;\n"
    "varying vec4 v_col;\n"
    "varying vec2 v_uv;\n"
    "void main(){\n"
    "  gl_Position = u_mvp * vec4(a_pos, 1.0);\n"
    "  gl_PointSize = u_psize;\n"
    "  v_col = a_col;\n"
    "  v_uv = a_uv;\n"
    "}\n";

static const char* IKS_FSH =
    "#version 100\n"
    "precision mediump float;\n"
    "varying vec4 v_col;\n"
    "varying vec2 v_uv;\n"
    "uniform sampler2D u_tex;\n"
    "void main(){\n"
    "  gl_FragColor = v_col * texture2D(u_tex, v_uv);\n"
    "}\n";

static GLuint iks_compile_shader(GLenum type, const char* src) {
    GLuint sh = glCreateShader(type);
    glShaderSource(sh, 1, &src, NULL);
    glCompileShader(sh);
    GLint ok = 0;
    glGetShaderiv(sh, GL_COMPILE_STATUS, &ok);
    if (!ok) {
        char log[1024];
        glGetShaderInfoLog(sh, sizeof log, NULL, log);
        IKERR("shader compile: %s", log);
        glDeleteShader(sh);
        return 0;
    }
    return sh;
}

static int iks_link_program(void) {
    GLuint vs = iks_compile_shader(GL_VERTEX_SHADER, IKS_VSH);
    GLuint fs = iks_compile_shader(GL_FRAGMENT_SHADER, IKS_FSH);
    if (!vs || !fs) return 0;
    GLuint p = glCreateProgram();
    glAttachShader(p, vs);
    glAttachShader(p, fs);
    glLinkProgram(p);
    glDeleteShader(vs);
    glDeleteShader(fs);
    GLint ok = 0;
    glGetProgramiv(p, GL_LINK_STATUS, &ok);
    if (!ok) {
        char log[1024];
        glGetProgramInfoLog(p, sizeof log, NULL, log);
        IKERR("program link: %s", log);
        return 0;
    }
    iks_prog = p;
    iks_a_pos = glGetAttribLocation(p, "a_pos");
    iks_a_col = glGetAttribLocation(p, "a_col");
    iks_a_uv = glGetAttribLocation(p, "a_uv");
    iks_u_mvp = glGetUniformLocation(p, "u_mvp");
    iks_u_psize = glGetUniformLocation(p, "u_psize");
    iks_u_tex = glGetUniformLocation(p, "u_tex");
    return 1;
}

static void iks_make_white_tex(void) {
    unsigned char px[4] = {255, 255, 255, 255};
    glGenTextures(1, &iks_white_tex);
    glBindTexture(GL_TEXTURE_2D, iks_white_tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, 1, 1, 0, GL_RGBA, GL_UNSIGNED_BYTE, px);
}

static void iks_glstate_init(void) {
    if (iks_glstate_ok) return;
    if (!iks_link_program()) { IKERR("gl init failed"); return; }
    iks_make_white_tex();
    iks_pmat = iks_ident();
    iks_mmat = iks_ident();
    glDisable(GL_DEPTH_TEST);
    glDepthFunc(GL_LEQUAL);
    glClearColor(0.0f, 0.0f, 0.0f, 1.0f);
    glDisable(GL_BLEND);
    glActiveTexture(GL_TEXTURE0);
    iks_glstate_ok = 1;
    IKALOG("gl state ready %dx%d", iks_win_w, iks_win_h);
}

/* ---- immediate-mode batch ---- */
typedef struct { float x, y, z; unsigned char r, g, b, a; float u, v; } IksGV;
static IksGV* iks_vbuf = NULL;
static int iks_vn = 0, iks_vcap = 0;
static IksGV* iks_dbuf = NULL;
static int iks_dn = 0, iks_dcap = 0;
static int iks_draw_mode = 0;
static int iks_in_begin = 0;
static unsigned char iks_cr = 255, iks_cg = 255, iks_cb = 255, iks_ca = 255;
static float iks_cu = 0.0f, iks_cv = 0.0f;

static void iks_dpush(IksGV v) {
    if (iks_dn >= iks_dcap) {
        int nc = iks_dcap ? iks_dcap * 2 : 4096;
        IksGV* nb = (IksGV*)realloc(iks_dbuf, (size_t)nc * sizeof(IksGV));
        if (!nb) { IKERR("oom"); exit(1); }
        iks_dbuf = nb;
        iks_dcap = nc;
    }
    iks_dbuf[iks_dn++] = v;
}

static void iks_draw_flush(void) {
    if (iks_vn <= 0 || !iks_glstate_ok) return;
    glUseProgram(iks_prog);
    IksM mvp = iks_mmul(iks_pmat, iks_mmat);
    glUniformMatrix4fv(iks_u_mvp, 1, GL_FALSE, mvp.m);
    glUniform1f(iks_u_psize, iks_psize);
    glUniform1i(iks_u_tex, 0);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_2D, iks_cur_tex ? iks_cur_tex : iks_white_tex);

    const IksGV* buf = iks_vbuf;
    int count = iks_vn;
    int gm;
    switch (iks_draw_mode) {
        case 0: gm = GL_POINTS; break;
        case 1: gm = GL_LINES; break;
        case 2: gm = GL_LINE_STRIP; break;
        case 4: gm = GL_TRIANGLE_STRIP; break;
        case 5: gm = GL_TRIANGLE_FAN; break;
        case 6: { /* GL_QUADS -> triangles */
            iks_dn = 0;
            for (int q = 0; q + 3 < iks_vn; q += 4) {
                IksGV a = iks_vbuf[q], b = iks_vbuf[q+1], c = iks_vbuf[q+2], d = iks_vbuf[q+3];
                iks_dpush(a); iks_dpush(b); iks_dpush(c);
                iks_dpush(a); iks_dpush(c); iks_dpush(d);
            }
            buf = iks_dbuf; count = iks_dn; gm = GL_TRIANGLES; break;
        }
        case 7: { /* GL_QUAD_STRIP -> triangles */
            iks_dn = 0;
            for (int q = 0; q + 3 < iks_vn; q += 2) {
                IksGV a = iks_vbuf[q], b = iks_vbuf[q+1], c = iks_vbuf[q+3], d = iks_vbuf[q+2];
                iks_dpush(a); iks_dpush(b); iks_dpush(c);
                iks_dpush(a); iks_dpush(c); iks_dpush(d);
            }
            buf = iks_dbuf; count = iks_dn; gm = GL_TRIANGLES; break;
        }
        default: gm = GL_TRIANGLES; break;
    }
    if (count <= 0) return;
    glEnableVertexAttribArray((GLuint)iks_a_pos);
    glEnableVertexAttribArray((GLuint)iks_a_col);
    glEnableVertexAttribArray((GLuint)iks_a_uv);
    glBindBuffer(GL_ARRAY_BUFFER, 0);
    glVertexAttribPointer((GLuint)iks_a_pos, 3, GL_FLOAT, GL_FALSE, sizeof(IksGV), &buf[0].x);
    glVertexAttribPointer((GLuint)iks_a_col, 4, GL_UNSIGNED_BYTE, GL_TRUE, sizeof(IksGV), &buf[0].r);
    glVertexAttribPointer((GLuint)iks_a_uv, 2, GL_FLOAT, GL_FALSE, sizeof(IksGV), &buf[0].u);
    glDrawArrays((GLenum)gm, 0, count);
    glDisableVertexAttribArray((GLuint)iks_a_pos);
    glDisableVertexAttribArray((GLuint)iks_a_col);
    glDisableVertexAttribArray((GLuint)iks_a_uv);
}

static void iks_gl_vertex_uv(double x, double y, double z, float u, float v) {
    if (!iks_in_begin) return;
    if (iks_vn >= iks_vcap) {
        int nc = iks_vcap ? iks_vcap * 2 : 4096;
        IksGV* nb = (IksGV*)realloc(iks_vbuf, (size_t)nc * sizeof(IksGV));
        if (!nb) { IKERR("oom"); exit(1); }
        iks_vbuf = nb;
        iks_vcap = nc;
    }
    IksGV* p = &iks_vbuf[iks_vn++];
    p->x = (float)x; p->y = (float)y; p->z = (float)z;
    p->r = iks_cr; p->g = iks_cg; p->b = iks_cb; p->a = iks_ca;
    p->u = u; p->v = v;
}

static void iks_gl_init(int64_t w, int64_t h, const char* title) {
    (void)title;
    if (w > 0 && h > 0) {
        if (!iks_win_w) iks_win_w = (int)w;
        if (!iks_win_h) iks_win_h = (int)h;
    }
    iks_glstate_init();
}
static void iks_gl_clear(int64_t r, int64_t g, int64_t b) {
    glClearColor((float)iks_clamp8(r)/255.0f, (float)iks_clamp8(g)/255.0f, (float)iks_clamp8(b)/255.0f, 1.0f);
    glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
}
static void iks_gl_view(int64_t x, int64_t y, int64_t w, int64_t h) {
    glViewport((GLint)x, (GLint)y, (GLsizei)w, (GLsizei)h);
}
static void iks_gl_proj(double fov, double aspect, double zn, double zf) {
    if (zn <= 0.0 || zf <= zn) { IKERR("bad frustum (need 0 < near < far)"); exit(1); }
    double top = zn * tan(fov * 3.14159265358979323846 / 360.0);
    iks_pmat = iks_mfrustum(-top*aspect, top*aspect, -top, top, zn, zf);
}
static void iks_gl_proj_ortho(double l, double r, double b, double t) {
    iks_pmat = iks_mortho(l, r, b, t, -1.0, 1.0);
}
static void iks_gl_push(void) { if (iks_mtop < 64) iks_mstack[iks_mtop++] = iks_mmat; }
static void iks_gl_pop(void) { if (iks_mtop > 0) iks_mmat = iks_mstack[--iks_mtop]; }
static void iks_gl_translate(double x, double y, double z) { iks_mmat = iks_mmul(iks_mmat, iks_mtrans(x, y, z)); }
static void iks_gl_rotate(double a, double x, double y, double z) { iks_mmat = iks_mmul(iks_mmat, iks_mrot(a, x, y, z)); }
static void iks_gl_scale(double x, double y, double z) { iks_mmat = iks_mmul(iks_mmat, iks_mscale(x, y, z)); }
static void iks_gl_color(int64_t r, int64_t g, int64_t b, int64_t a) {
    iks_cr = iks_clamp8(r); iks_cg = iks_clamp8(g); iks_cb = iks_clamp8(b); iks_ca = iks_clamp8(a);
}
static void iks_gl_begin(int64_t mode) { iks_draw_mode = (int)mode; iks_vn = 0; iks_in_begin = 1; }
static void iks_gl_vertex(double x, double y, double z) { iks_gl_vertex_uv(x, y, z, iks_cu, iks_cv); }
static void iks_gl_end(void) {
    if (!iks_in_begin) return;
    iks_in_begin = 0;
    if (iks_vn > 0) iks_draw_flush();
    iks_vn = 0;
}
static void iks_gl_rect(double x, double y, double w, double h) {
    iks_gl_begin(6);
    iks_gl_vertex(x, y, 0); iks_gl_vertex(x+w, y, 0); iks_gl_vertex(x+w, y+h, 0); iks_gl_vertex(x, y+h, 0);
    iks_gl_end();
}
static void iks_gl_circle(double cx, double cy, double r, int64_t segs) {
    if (segs < 3) segs = 32;
    iks_gl_begin(5);
    iks_gl_vertex(cx, cy, 0);
    double step = 6.283185307179586 / (double)segs;
    for (int64_t i = 0; i <= segs; i++) {
        double a = (double)i * step;
        iks_gl_vertex(cx + cos(a)*r, cy + sin(a)*r, 0);
    }
    iks_gl_end();
}
static void iks_gl_line(double x1, double y1, double x2, double y2) {
    iks_gl_begin(1);
    iks_gl_vertex(x1, y1, 0);
    iks_gl_vertex(x2, y2, 0);
    iks_gl_end();
}
static void iks_gl_point_size(double s) { iks_psize = (float)s; }
static void iks_gl_line_width(double w) { glLineWidth((GLfloat)w); }
static void iks_gl_depth(int64_t on) { if (on) glEnable(GL_DEPTH_TEST); else glDisable(GL_DEPTH_TEST); }
static void iks_gl_blend(int64_t on) {
    if (on) { glEnable(GL_BLEND); glBlendFunc(GL_ONE, GL_ONE); }
    else glDisable(GL_BLEND);
}
static void iks_gl_title(const char* t) { (void)t; }
static void iks_gl_fullscreen(int64_t on) { (void)on; }
static int64_t iks_gl_width(void) { return iks_win_w ? iks_win_w : iks_gw; }
static int64_t iks_gl_height(void) { return iks_win_h ? iks_win_h : iks_gh; }
static int64_t iks_gl_swap(void) {
    if (iks_esurf != EGL_NO_SURFACE) eglSwapBuffers(iks_edpy, iks_esurf);
    return iks_gfx_poll();
}

/* ---- software framebuffer (gfx_*) ---- */
static void iks_gfx_open(int64_t w, int64_t h, const char* title) {
    (void)title;
    if (iks_gfx_on) return;
    if (w < 64 || h < 64 || w > 3840 || h > 2160) { IKERR("gfx window size must be 64x64..3840x2160"); exit(1); }
    iks_gw = (int)w;
    iks_gh = (int)h;
    iks_pixels = calloc((size_t)iks_gw * (size_t)iks_gh, 4);
    if (!iks_pixels) { IKERR("out of memory"); exit(1); }
    glGenTextures(1, &iks_fb_tex);
    glBindTexture(GL_TEXTURE_2D, iks_fb_tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, iks_gw, iks_gh, 0, GL_RGBA, GL_UNSIGNED_BYTE, NULL);
    iks_gfx_t0 = iks_now();
    iks_gfx_on = 1;
}
static int64_t iks_gfx_poll(void) {
    if (iks_app && iks_app->destroyRequested) return 0;
    int ev;
    struct android_poll_source* src;
    while (ALooper_pollOnce(0, NULL, &ev, (void**)&src) >= 0 && src) {
        src->process(iks_app, src);
    }
    if (iks_gfx_closed) return 0;
    return 1;
}
static void iks_gfx_present(void) {
    if (!iks_gfx_on || !iks_pixels || !iks_glstate_ok) return;
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_2D, iks_fb_tex);
    glTexSubImage2D(GL_TEXTURE_2D, 0, 0, 0, iks_gw, iks_gh, GL_RGBA, GL_UNSIGNED_BYTE, iks_pixels);
    IksM sp = iks_pmat, sm = iks_mmat;
    unsigned char pr = iks_cr, pg = iks_cg, pb = iks_cb, pa = iks_ca;
    GLuint st = iks_cur_tex;
    float pu = iks_cu, pv = iks_cv;
    iks_pmat = iks_mortho(0, iks_gw, iks_gh, 0, -1.0, 1.0);
    iks_mmat = iks_ident();
    iks_cur_tex = iks_fb_tex;
    iks_cr = iks_cg = iks_cb = iks_ca = 255;
    iks_gl_begin(6);
    iks_gl_vertex_uv(0, 0, 0, 0, 0);
    iks_gl_vertex_uv(iks_gw, 0, 0, 1, 0);
    iks_gl_vertex_uv(iks_gw, iks_gh, 0, 1, 1);
    iks_gl_vertex_uv(0, iks_gh, 0, 0, 1);
    iks_gl_end();
    iks_pmat = sp; iks_mmat = sm; iks_cur_tex = st;
    iks_cr = pr; iks_cg = pg; iks_cb = pb; iks_ca = pa;
    iks_cu = pu; iks_cv = pv;
}
static void iks_gfx_clear(int64_t r, int64_t g, int64_t b) {
    if (!iks_pixels) return;
    unsigned char cr = iks_clamp8(r), cg = iks_clamp8(g), cb = iks_clamp8(b);
    unsigned char* p = (unsigned char*)iks_pixels;
    size_t n = (size_t)iks_gw * (size_t)iks_gh;
    for (size_t i = 0; i < n; i++) { p[0] = cr; p[1] = cg; p[2] = cb; p[3] = 255; p += 4; }
}
static void iks_gfx_pixel(int64_t x, int64_t y, int64_t r, int64_t g, int64_t b) {
    if (!iks_pixels) return;
    if (x < 0 || y < 0 || x >= iks_gw || y >= iks_gh) return;
    unsigned char* p = (unsigned char*)iks_pixels + (y * iks_gw + x) * 4;
    p[0] = iks_clamp8(r); p[1] = iks_clamp8(g); p[2] = iks_clamp8(b); p[3] = 255;
}
static void iks_gfx_add(int64_t x, int64_t y, int64_t r, int64_t g, int64_t b) {
    if (!iks_pixels) return;
    if (x < 0 || y < 0 || x >= iks_gw || y >= iks_gh) return;
    unsigned char* p = (unsigned char*)iks_pixels + (y * iks_gw + x) * 4;
    int rr = (int)p[0] + (int)r;
    int gg = (int)p[1] + (int)g;
    int bb = (int)p[2] + (int)b;
    p[0] = (unsigned char)(rr > 255 ? 255 : rr);
    p[1] = (unsigned char)(gg > 255 ? 255 : gg);
    p[2] = (unsigned char)(bb > 255 ? 255 : bb);
}
static void iks_gfx_line(int64_t x0, int64_t y0, int64_t x1, int64_t y1,
                         int64_t r, int64_t g, int64_t b) {
    int64_t dx = x1 > x0 ? x1 - x0 : x0 - x1;
    int64_t dy = y1 > y0 ? y1 - y0 : y0 - y1;
    int64_t sx = x0 < x1 ? 1 : -1;
    int64_t sy = y0 < y1 ? 1 : -1;
    int64_t err = dx - dy;
    for (;;) {
        iks_gfx_pixel(x0, y0, r, g, b);
        if (x0 == x1 && y0 == y1) break;
        int64_t e2 = 2 * err;
        if (e2 > -dy) { err -= dy; x0 += sx; }
        if (e2 < dx) { err += dx; y0 += sy; }
    }
}
static int64_t iks_gfx_width(void) { return iks_gw; }
static int64_t iks_gfx_height(void) { return iks_gh; }
static double iks_gfx_time(void) { return iks_now() - iks_gfx_t0; }
static int64_t iks_gfx_mouse_x(void) { return iks_mx; }
static int64_t iks_gfx_mouse_y(void) { return iks_my; }
static int64_t iks_gfx_key_down(int64_t vk) { if (vk < 0 || vk >= 256) return 0; return iks_keys[vk] ? 1 : 0; }
static int64_t iks_gfx_mouse_button(int64_t b) { if (b < 0 || b >= 3) return 0; return iks_mbtn[b] ? 1 : 0; }
static void iks_sleep(int64_t ms) { if (ms > 0) usleep((useconds_t)(ms * 1000)); }
static double iks_gpu_load(void) { return 0.0; }
static double iks_cpu_load(void) { return 0.0; }
static int64_t iks_npu_available(void) { return 0; }
static int64_t iks_npu_count(void) { return 0; }
static char* iks_npu_name(void) { return ""; }
static double iks_npu_tops(void) { return 0.0; }
static double iks_npu_load(void) { return 0.0; }

/* ---- input / lifecycle ---- */
static int32_t iks_on_input(struct android_app* app, AInputEvent* ev) {
    (void)app;
    if (AInputEvent_getType(ev) != AINPUT_EVENT_TYPE_MOTION) return 0;
    iks_mx = (int)AMotionEvent_getX(ev, 0);
    iks_my = (int)AMotionEvent_getY(ev, 0);
    int32_t action = AMotionEvent_getAction(ev) & AMOTION_EVENT_ACTION_MASK;
    if (action == AMOTION_EVENT_ACTION_DOWN || action == AMOTION_EVENT_ACTION_POINTER_DOWN)
        iks_mbtn[0] = 1;
    else if (action == AMOTION_EVENT_ACTION_UP || action == AMOTION_EVENT_ACTION_POINTER_UP ||
             action == AMOTION_EVENT_ACTION_CANCEL)
        iks_mbtn[0] = 0;
    return 1;
}

static void iks_egl_ensure(void) {
    if (iks_egl_inited) return;
    iks_edpy = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    if (iks_edpy == EGL_NO_DISPLAY) { IKERR("no EGL display"); return; }
    if (!eglInitialize(iks_edpy, NULL, NULL)) { IKERR("eglInitialize failed"); return; }
    const EGLint attr[] = {
        EGL_SURFACE_TYPE, EGL_WINDOW_BIT,
        EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
        EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8, EGL_ALPHA_SIZE, 8,
        EGL_DEPTH_SIZE, 24, EGL_NONE
    };
    EGLint n = 0;
    if (!eglChooseConfig(iks_edpy, attr, &iks_ecfg, 1, &n) || n < 1) { IKERR("no EGL config"); return; }
    const EGLint ctx[] = { EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE };
    iks_ectx = eglCreateContext(iks_edpy, iks_ecfg, EGL_NO_CONTEXT, ctx);
    if (iks_ectx == EGL_NO_CONTEXT) { IKERR("eglCreateContext failed"); return; }
    iks_egl_inited = 1;
}

static void iks_surf_create(ANativeWindow* win) {
    iks_egl_ensure();
    if (!iks_egl_inited || iks_esurf != EGL_NO_SURFACE) return;
    EGLint fmt = 0;
    eglGetConfigAttrib(iks_edpy, iks_ecfg, EGL_NATIVE_VISUAL_ID, &fmt);
    ANativeWindow_setBuffersGeometry(win, 0, 0, fmt);
    iks_esurf = eglCreateWindowSurface(iks_edpy, iks_ecfg, win, NULL);
    if (iks_esurf == EGL_NO_SURFACE) { IKERR("eglCreateWindowSurface failed: 0x%x", eglGetError()); return; }
    if (!eglMakeCurrent(iks_edpy, iks_esurf, iks_esurf, iks_ectx)) { IKERR("eglMakeCurrent failed"); return; }
    eglSwapInterval(iks_edpy, 0);
    EGLint w = 0, h = 0;
    eglQuerySurface(iks_edpy, iks_esurf, EGL_WIDTH, &w);
    eglQuerySurface(iks_edpy, iks_esurf, EGL_HEIGHT, &h);
    iks_win_w = w; iks_win_h = h;
    iks_glstate_init();
}

static void iks_surf_destroy(void) {
    if (iks_esurf != EGL_NO_SURFACE) {
        eglMakeCurrent(iks_edpy, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
        eglDestroySurface(iks_edpy, iks_esurf);
        iks_esurf = EGL_NO_SURFACE;
    }
}

static void iks_on_cmd(struct android_app* app, int32_t cmd) {
    switch (cmd) {
        case APP_CMD_INIT_WINDOW:
            if (app->window) iks_surf_create(app->window);
            break;
        case APP_CMD_TERM_WINDOW:
            iks_surf_destroy();
            break;
        case APP_CMD_CONFIG_CHANGED:
            if (iks_esurf != EGL_NO_SURFACE) {
                EGLint w = 0, h = 0;
                eglQuerySurface(iks_edpy, iks_esurf, EGL_WIDTH, &w);
                eglQuerySurface(iks_edpy, iks_esurf, EGL_HEIGHT, &h);
                if (w) iks_win_w = w;
                if (h) iks_win_h = h;
            }
            break;
        default:
            break;
    }
}

void android_main(struct android_app* app) {
    iks_app = app;
    app->onAppCmd = iks_on_cmd;
    app->onInputEvent = iks_on_input;
    iks_gfx_t0 = iks_now();
    IKALOG("android_main start");
    while (iks_esurf == EGL_NO_SURFACE && !app->destroyRequested) {
        int ev;
        struct android_poll_source* src;
        int r = ALooper_pollOnce(-1, NULL, &ev, (void**)&src);
        if (r >= 0 && src) src->process(app, src);
    }
    if (iks_esurf != EGL_NO_SURFACE) {
        iks_glstate_init();
        main();
    }
    iks_surf_destroy();
    if (iks_edpy != EGL_NO_DISPLAY) {
        eglMakeCurrent(iks_edpy, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
        if (iks_ectx != EGL_NO_CONTEXT) eglDestroyContext(iks_edpy, iks_ectx);
        eglTerminate(iks_edpy);
    }
    IKALOG("android_main exit");
}

/* ---- non-GLES backends are unavailable on Android ---- */
static void iks_d3d_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d11 error: not available on Android\n"); exit(1); }
static void iks_d3d_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d3d_draw(IksArray a) { (void)a; }
static int64_t iks_d3d_present(void) { return 0; }
static void iks_d3d_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d3d_begin(int64_t m) { (void)m; }
static void iks_d3d_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d3d_end(void) {}
static void iks_d3d_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d3d_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d3d_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static void iks_d3d_title(const char* t) { (void)t; }
static int64_t iks_d3d_width(void) { return 0; }
static int64_t iks_d3d_height(void) { return 0; }
static void iks_d12_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d12 error: not available on Android\n"); exit(1); }
static void iks_d12_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d12_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d12_begin(int64_t m) { (void)m; }
static void iks_d12_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d12_end(void) {}
static void iks_d12_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d12_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d12_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_d12_swap(void) { return 0; }
static void iks_d12_title(const char* t) { (void)t; }
static int64_t iks_d12_width(void) { return 0; }
static int64_t iks_d12_height(void) { return 0; }
static void iks_d9_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d9 error: not available on Android\n"); exit(1); }
static void iks_d9_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d9_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d9_begin(int64_t m) { (void)m; }
static void iks_d9_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d9_end(void) {}
static void iks_d9_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d9_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d9_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_d9_swap(void) { return 0; }
static void iks_d9_title(const char* t) { (void)t; }
static int64_t iks_d9_width(void) { return 0; }
static int64_t iks_d9_height(void) { return 0; }
static void iks_d10_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d10 error: not available on Android\n"); exit(1); }
static void iks_d10_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d10_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d10_begin(int64_t m) { (void)m; }
static void iks_d10_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d10_end(void) {}
static void iks_d10_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d10_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d10_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_d10_swap(void) { return 0; }
static void iks_d10_title(const char* t) { (void)t; }
static int64_t iks_d10_width(void) { return 0; }
static int64_t iks_d10_height(void) { return 0; }
static void iks_vk_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "vulkan error: not available on Android GLES build\n"); exit(1); }
static void iks_vk_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_vk_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_vk_begin(int64_t m) { (void)m; }
static void iks_vk_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_vk_end(void) {}
static void iks_vk_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_vk_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_vk_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_vk_swap(void) { return 0; }
static void iks_vk_title(const char* t) { (void)t; }
static int64_t iks_vk_width(void) { return 0; }
static int64_t iks_vk_height(void) { return 0; }

#elif defined(IKS_LINUX)

#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/keysym.h>
#include <GL/gl.h>
#include <GL/glx.h>
#include <unistd.h>
#include <sys/time.h>
#include <dirent.h>

static Display* iks_dpy;
static Window iks_win;
static GC iks_gc;
static Colormap iks_cmap;
static XVisualInfo* iks_vi;
static XImage* iks_img;
static Atom iks_wm_delete;
static void* iks_pixels;
static int iks_gw = 0, iks_gh = 0;
static int iks_gfx_on = 0, iks_gfx_closed = 0;
static double iks_gfx_t0 = 0.0;
static unsigned char iks_keys[256];
static int iks_mx = 0, iks_my = 0;
static unsigned char iks_mbtn[3];
static GLXContext iks_glctx;

static double iks_now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

static unsigned char iks_clamp8(int64_t v) {
    if (v < 0) return 0;
    if (v > 255) return 255;
    return (unsigned char)v;
}

static int iks_keysym_to_vk(KeySym ks) {
    if (ks >= XK_a && ks <= XK_z) return (int)(ks - XK_a + 0x41);
    if (ks >= XK_0 && ks <= XK_9) return (int)(ks - XK_0 + 0x30);
    if (ks >= XK_F1 && ks <= XK_F12) return 0x70 + (int)(ks - XK_F1);
    switch (ks) {
        case XK_Escape: return 0x1B;
        case XK_space: return 0x20;
        case XK_Return: case XK_KP_Enter: return 0x0D;
        case XK_Tab: case XK_ISO_Left_Tab: return 0x09;
        case XK_BackSpace: return 0x08;
        case XK_Left: return 0x25;
        case XK_Up: return 0x26;
        case XK_Right: return 0x27;
        case XK_Down: return 0x28;
        case XK_Shift_L: case XK_Shift_R: return 0x10;
        case XK_Control_L: case XK_Control_R: return 0x11;
        case XK_Alt_L: case XK_Alt_R: case XK_Meta_L: case XK_Meta_R: return 0x12;
        case XK_Delete: return 0x2E;
        case XK_Home: return 0x24;
        case XK_End: return 0x23;
        case XK_Page_Up: return 0x21;
        case XK_Page_Down: return 0x22;
        case XK_Insert: return 0x2D;
        case XK_Caps_Lock: return 0x14;
        case XK_Pause: return 0x13;
        case XK_Scroll_Lock: return 0x91;
        case XK_Print: return 0x2C;
        case XK_Num_Lock: return 0x90;
        default: return 0;
    }
}

static void iks_gfx_open(int64_t w, int64_t h, const char* title) {
    if (iks_gfx_on) return;
    if (w < 64 || h < 64 || w > 3840 || h > 2160) {
        fprintf(stderr, "gfx error: window size must be between 64x64 and 3840x2160\n");
        exit(1);
    }
    iks_dpy = XOpenDisplay(NULL);
    if (!iks_dpy) {
        fprintf(stderr, "gfx error: cannot open X display (is DISPLAY set? on WSL use WSLg)\n");
        exit(1);
    }
    int screen = DefaultScreen(iks_dpy);
    static int glattribs[] = {
        GLX_RGBA, GLX_DOUBLEBUFFER,
        GLX_RED_SIZE, 8, GLX_GREEN_SIZE, 8, GLX_BLUE_SIZE, 8,
        GLX_DEPTH_SIZE, 24, None
    };
    iks_vi = glXChooseVisual(iks_dpy, screen, glattribs);
    if (!iks_vi) {
        XVisualInfo tmpl;
        int n = 0;
        memset(&tmpl, 0, sizeof(tmpl));
        tmpl.screen = screen;
        iks_vi = XGetVisualInfo(iks_dpy, VisualScreenMask, &tmpl, &n);
    }
    if (!iks_vi) {
        fprintf(stderr, "gfx error: no usable X visual\n");
        exit(1);
    }
    iks_gw = (int)w;
    iks_gh = (int)h;
    iks_cmap = XCreateColormap(iks_dpy, RootWindow(iks_dpy, screen), iks_vi->visual, AllocNone);
    XSetWindowAttributes swa;
    memset(&swa, 0, sizeof(swa));
    swa.colormap = iks_cmap;
    swa.background_pixel = 0;
    swa.event_mask = KeyPressMask | KeyReleaseMask | ButtonPressMask | ButtonReleaseMask |
                     PointerMotionMask | ExposureMask | StructureNotifyMask;
    iks_win = XCreateWindow(iks_dpy, RootWindow(iks_dpy, screen), 0, 0,
                            (unsigned)iks_gw, (unsigned)iks_gh, 0, iks_vi->depth,
                            InputOutput, iks_vi->visual,
                            CWColormap | CWBackPixel | CWEventMask, &swa);
    XStoreName(iks_dpy, iks_win, title ? title : "IKS");
    iks_wm_delete = XInternAtom(iks_dpy, "WM_DELETE_WINDOW", False);
    XSetWMProtocols(iks_dpy, iks_win, &iks_wm_delete, 1);
    iks_gc = XCreateGC(iks_dpy, iks_win, 0, NULL);
    iks_pixels = calloc((size_t)iks_gw * (size_t)iks_gh, 4);
    if (!iks_pixels) {
        fprintf(stderr, "gfx error: out of memory\n");
        exit(1);
    }
    iks_img = XCreateImage(iks_dpy, iks_vi->visual, (unsigned)iks_vi->depth, ZPixmap, 0,
                           (char*)iks_pixels, (unsigned)iks_gw, (unsigned)iks_gh, 32, 0);
    if (!iks_img || iks_img->bytes_per_line != iks_gw * 4) {
        fprintf(stderr, "gfx error: unsupported X image layout\n");
        exit(1);
    }
    XMapWindow(iks_dpy, iks_win);
    XFlush(iks_dpy);
    iks_gfx_t0 = iks_now();
    iks_gfx_on = 1;
}

static int64_t iks_gfx_poll(void) {
    if (!iks_gfx_on || iks_gfx_closed) return 0;
    while (iks_dpy && XPending(iks_dpy) > 0) {
        XEvent ev;
        XNextEvent(iks_dpy, &ev);
        switch (ev.type) {
            case ClientMessage:
                if ((Atom)ev.xclient.data.l[0] == iks_wm_delete) iks_gfx_closed = 1;
                break;
            case DestroyNotify:
                iks_gfx_closed = 1;
                break;
            case KeyPress: {
                int vk = iks_keysym_to_vk(XLookupKeysym(&ev.xkey, 0));
                if (vk > 0 && vk < 256) iks_keys[vk] = 1;
                break;
            }
            case KeyRelease: {
                int vk = iks_keysym_to_vk(XLookupKeysym(&ev.xkey, 0));
                if (vk > 0 && vk < 256) iks_keys[vk] = 0;
                break;
            }
            case MotionNotify:
                iks_mx = ev.xmotion.x;
                iks_my = ev.xmotion.y;
                break;
            case ButtonPress:
                if (ev.xbutton.button == 1) iks_mbtn[0] = 1;
                else if (ev.xbutton.button == 3) iks_mbtn[1] = 1;
                else if (ev.xbutton.button == 2) iks_mbtn[2] = 1;
                break;
            case ButtonRelease:
                if (ev.xbutton.button == 1) iks_mbtn[0] = 0;
                else if (ev.xbutton.button == 3) iks_mbtn[1] = 0;
                else if (ev.xbutton.button == 2) iks_mbtn[2] = 0;
                break;
            default:
                break;
        }
    }
    return iks_gfx_closed ? 0 : 1;
}

static void iks_gfx_present(void) {
    if (!iks_gfx_on || !iks_img || !iks_dpy) return;
    XPutImage(iks_dpy, iks_win, iks_gc, iks_img, 0, 0, 0, 0,
              (unsigned)iks_gw, (unsigned)iks_gh);
    XFlush(iks_dpy);
}

static void iks_gfx_clear(int64_t r, int64_t g, int64_t b) {
    if (!iks_pixels) return;
    unsigned char cr = iks_clamp8(r), cg = iks_clamp8(g), cb = iks_clamp8(b);
    unsigned int px = (unsigned int)cb | ((unsigned int)cg << 8) | ((unsigned int)cr << 16);
    unsigned int* p = (unsigned int*)iks_pixels;
    size_t n = (size_t)iks_gw * (size_t)iks_gh;
    for (size_t i = 0; i < n; i++) p[i] = px;
}

static void iks_gfx_pixel(int64_t x, int64_t y, int64_t r, int64_t g, int64_t b) {
    if (!iks_pixels) return;
    if (x < 0 || y < 0 || x >= iks_gw || y >= iks_gh) return;
    unsigned char* p = (unsigned char*)iks_pixels + (y * iks_gw + x) * 4;
    p[0] = iks_clamp8(b);
    p[1] = iks_clamp8(g);
    p[2] = iks_clamp8(r);
}

static void iks_gfx_add(int64_t x, int64_t y, int64_t r, int64_t g, int64_t b) {
    if (!iks_pixels) return;
    if (x < 0 || y < 0 || x >= iks_gw || y >= iks_gh) return;
    unsigned char* p = (unsigned char*)iks_pixels + (y * iks_gw + x) * 4;
    int rr = (int)p[2] + (int)r;
    int gg = (int)p[1] + (int)g;
    int bb = (int)p[0] + (int)b;
    p[2] = (unsigned char)(rr > 255 ? 255 : rr);
    p[1] = (unsigned char)(gg > 255 ? 255 : gg);
    p[0] = (unsigned char)(bb > 255 ? 255 : bb);
}

static void iks_gfx_line(int64_t x0, int64_t y0, int64_t x1, int64_t y1,
                         int64_t r, int64_t g, int64_t b) {
    int64_t dx = x1 > x0 ? x1 - x0 : x0 - x1;
    int64_t dy = y1 > y0 ? y1 - y0 : y0 - y1;
    int64_t sx = x0 < x1 ? 1 : -1;
    int64_t sy = y0 < y1 ? 1 : -1;
    int64_t err = dx - dy;
    for (;;) {
        iks_gfx_pixel(x0, y0, r, g, b);
        if (x0 == x1 && y0 == y1) break;
        int64_t e2 = 2 * err;
        if (e2 > -dy) {
            err -= dy;
            x0 += sx;
        }
        if (e2 < dx) {
            err += dx;
            y0 += sy;
        }
    }
}

static int64_t iks_gfx_width(void) { return iks_gw; }
static int64_t iks_gfx_height(void) { return iks_gh; }
static double iks_gfx_time(void) { return iks_now() - iks_gfx_t0; }
static int64_t iks_gfx_mouse_x(void) { return iks_mx; }
static int64_t iks_gfx_mouse_y(void) { return iks_my; }

static int64_t iks_gfx_key_down(int64_t vk) {
    if (vk < 0 || vk >= 256) return 0;
    return iks_keys[vk] ? 1 : 0;
}

static int64_t iks_gfx_mouse_button(int64_t b) {
    if (b < 0 || b >= 3) return 0;
    return iks_mbtn[b] ? 1 : 0;
}

static void iks_sleep(int64_t ms) {
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (ms % 1000) * 1000000L;
    nanosleep(&ts, NULL);
}

static double iks_cpu_load(void) {
    static unsigned long long last_total = 0, last_idle = 0;
    FILE* f = fopen("/proc/stat", "r");
    if (!f) return 0.0;
    char line[512];
    if (!fgets(line, sizeof(line), f)) {
        fclose(f);
        return 0.0;
    }
    fclose(f);
    unsigned long long user = 0, nice = 0, system = 0, idle = 0;
    unsigned long long iowait = 0, irq = 0, softirq = 0, steal = 0;
    if (sscanf(line, "cpu %llu %llu %llu %llu %llu %llu %llu %llu",
               &user, &nice, &system, &idle, &iowait, &irq, &softirq, &steal) < 4) {
        return 0.0;
    }
    unsigned long long idle_all = idle + iowait;
    unsigned long long total = user + nice + system + idle + iowait + irq + softirq + steal;
    double pct = 0.0;
    if (last_total > 0 && total > last_total) {
        double dt = (double)(total - last_total);
        double di = (double)(idle_all - last_idle);
        pct = (1.0 - di / dt) * 100.0;
        if (pct < 0.0) pct = 0.0;
        if (pct > 100.0) pct = 100.0;
    }
    last_total = total;
    last_idle = idle_all;
    return pct;
}

static double iks_gpu_load(void) {
    static const char* paths[] = {
        "/sys/class/drm/card0/gpu_busy_percent",
        "/sys/class/drm/card1/gpu_busy_percent",
        "/sys/class/drm/card0/device/gpu_busy_percent",
        "/sys/class/drm/card1/device/gpu_busy_percent",
        NULL
    };
    for (int i = 0; paths[i]; i++) {
        FILE* f = fopen(paths[i], "r");
        if (!f) continue;
        double v = 0.0;
        int got = fscanf(f, "%lf", &v);
        fclose(f);
        if (got == 1 && v >= 0.0 && v <= 100.0) return v;
    }
    return 0.0;
}

/* ---- NPU detection: sysfs description heuristic (no device-ID guessing) ---- */
static int iks_npu_n = -1;
static char iks_npu_nm[192] = "";

static int iks_npu_word(const char* hay, const char* w) {
    size_t wl = strlen(w);
    const char* p = hay;
    while ((p = strstr(p, w)) != NULL) {
        char bc = (p == hay) ? 0 : p[-1];
        char ac = p[wl];
        int bok = !((bc >= 'a' && bc <= 'z') || (bc >= '0' && bc <= '9'));
        int aok = !((ac >= 'a' && ac <= 'z') || (ac >= '0' && ac <= '9'));
        if (bok && aok) return 1;
        p++;
    }
    return 0;
}

static int iks_npu_look(const char* s) {
    char b[2048];
    int i, n = 0;
    if (!s) return 0;
    for (i = 0; s[i] && n < 2047; i++) {
        char c = s[i];
        if (c >= 'A' && c <= 'Z') c = (char)(c - 'A' + 'a');
        b[n++] = c;
    }
    b[n] = 0;
    if (iks_npu_word(b, "npu")) return 1;
    if (strstr(b, "neural")) return 1;
    if (strstr(b, "xdna")) return 1;
    if (strstr(b, "hexagon")) return 1;
    if (strstr(b, "myriad")) return 1;
    if (strstr(b, "ai boost")) return 1;
    if (strstr(b, "ai accelerator")) return 1;
    if (strstr(b, "ai acceleration")) return 1;
    if (strstr(b, "ai engine")) return 1;
    return 0;
}

static void iks_npu_take(const char* s) {
    int n = 0;
    if (!s) s = "";
    while (*s && *s != '\n' && *s != '\r' && n < (int)sizeof(iks_npu_nm) - 1)
        iks_npu_nm[n++] = *s++;
    iks_npu_nm[n] = 0;
}

static void iks_npu_read(const char* path, char* out, int cap) {
    FILE* f = fopen(path, "r");
    size_t n;
    out[0] = 0;
    if (!f) return;
    n = fread(out, 1, (size_t)cap - 1, f);
    out[n] = 0;
    fclose(f);
}

static void iks_npu_scan(void) {
    DIR* d;
    struct dirent* e;
    char p[512], buf[2048];
    if (iks_npu_n >= 0) return;
    iks_npu_n = 0;
    iks_npu_nm[0] = 0;
    d = opendir("/sys/class/accel");
    if (d) {
        while ((e = readdir(d)) != NULL) {
            const char* dr;
            if (strncmp(e->d_name, "accel", 5) != 0) continue;
            if (iks_npu_n == 0) {
                snprintf(p, sizeof(p), "/sys/class/accel/%s/device/label", e->d_name);
                iks_npu_read(p, buf, (int)sizeof(buf));
                if (!buf[0]) {
                    snprintf(p, sizeof(p), "/sys/class/accel/%s/device/uevent", e->d_name);
                    iks_npu_read(p, buf, (int)sizeof(buf));
                }
                dr = strstr(buf, "DRIVER=");
                iks_npu_take(dr ? dr + 7 : (buf[0] ? buf : e->d_name));
            }
            iks_npu_n++;
        }
        closedir(d);
    }
    if (iks_npu_n > 0) return;
    d = opendir("/dev/accel");
    if (d) {
        while ((e = readdir(d)) != NULL) {
            if (strncmp(e->d_name, "accel", 5) != 0) continue;
            if (iks_npu_n == 0) iks_npu_take(e->d_name);
            iks_npu_n++;
        }
        closedir(d);
    }
    if (iks_npu_n > 0) return;
    d = opendir("/sys/bus/pci/devices");
    if (d) {
        while ((e = readdir(d)) != NULL) {
            const char* dr;
            if (e->d_name[0] == '.') continue;
            snprintf(p, sizeof(p), "/sys/bus/pci/devices/%s/uevent", e->d_name);
            iks_npu_read(p, buf, (int)sizeof(buf));
            if (!buf[0]) continue;
            if (!iks_npu_look(buf)) continue;
            if (iks_npu_n == 0) {
                dr = strstr(buf, "DRIVER=");
                iks_npu_take(dr ? dr + 7 : e->d_name);
            }
            iks_npu_n++;
        }
        closedir(d);
    }
}

static int64_t iks_npu_available(void) { iks_npu_scan(); return iks_npu_n > 0 ? 1 : 0; }
static int64_t iks_npu_count(void) { iks_npu_scan(); return (int64_t)iks_npu_n; }
static char* iks_npu_name(void) { iks_npu_scan(); return iks_npu_nm; }
static double iks_npu_tops(void) { return 0.0; }
static double iks_npu_load(void) { return 0.0; }

static void iks_gl_init(int64_t w, int64_t h, const char* title) {
    if (!iks_gfx_on) iks_gfx_open(w, h, title);
    if (iks_glctx) return;
    if (!iks_vi) {
        fprintf(stderr, "gfx error: no GLX visual\n");
        exit(1);
    }
    iks_glctx = glXCreateContext(iks_dpy, iks_vi, NULL, True);
    if (!iks_glctx || !glXMakeCurrent(iks_dpy, iks_win, iks_glctx)) {
        fprintf(stderr, "gfx error: cannot create OpenGL context\n");
        exit(1);
    }
    glEnable(GL_DEPTH_TEST);
    glDepthFunc(GL_LEQUAL);
    glClearColor(0.0f, 0.0f, 0.0f, 1.0f);
    typedef int (*iks_swap_interval_t)(Display*, GLXDrawable, int);
    iks_swap_interval_t fext =
        (iks_swap_interval_t)glXGetProcAddress((const GLubyte*)"glXSwapIntervalEXT");
    if (fext) {
        fext(iks_dpy, iks_win, 0);
    } else {
        typedef int (*iks_swap_interval_mesa_t)(unsigned);
        iks_swap_interval_mesa_t fmesa =
            (iks_swap_interval_mesa_t)glXGetProcAddress((const GLubyte*)"glXSwapIntervalMESA");
        if (fmesa) fmesa(0);
    }
}

static void iks_gl_clear(int64_t r, int64_t g, int64_t b) {
    glClearColor((float)iks_clamp8(r) / 255.0f,
                 (float)iks_clamp8(g) / 255.0f,
                 (float)iks_clamp8(b) / 255.0f, 1.0f);
    glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
}

static void iks_gl_view(int64_t x, int64_t y, int64_t w, int64_t h) {
    glViewport((GLint)x, (GLint)y, (GLsizei)w, (GLsizei)h);
}

static void iks_gl_proj(double fov, double aspect, double zn, double zf) {
    if (zn <= 0.0 || zf <= zn) {
        fprintf(stderr, "gfx error: bad frustum (need 0 < near < far)\n");
        exit(1);
    }
    double top = zn * tan(fov * 3.14159265358979323846 / 360.0);
    glMatrixMode(GL_PROJECTION);
    glLoadIdentity();
    glFrustum(-top * aspect, top * aspect, -top, top, zn, zf);
    glMatrixMode(GL_MODELVIEW);
}

static void iks_gl_push(void) { glPushMatrix(); }
static void iks_gl_pop(void) { glPopMatrix(); }
static void iks_gl_translate(double x, double y, double z) { glTranslated(x, y, z); }
static void iks_gl_rotate(double a, double x, double y, double z) { glRotated(a, x, y, z); }
static void iks_gl_scale(double x, double y, double z) { glScaled(x, y, z); }

typedef struct { float x, y, z; GLubyte r, g, b, a; } IksV;
static IksV* iks_vbuf = NULL;
static int iks_vn = 0;
static int iks_vcap = 0;
static int iks_draw_mode = 0;
static int iks_in_begin = 0;
static GLubyte iks_cr = 255, iks_cg = 255, iks_cb = 255, iks_ca = 255;

static void iks_gl_color(int64_t r, int64_t g, int64_t b, int64_t a) {
    iks_cr = (GLubyte)iks_clamp8(r);
    iks_cg = (GLubyte)iks_clamp8(g);
    iks_cb = (GLubyte)iks_clamp8(b);
    iks_ca = (GLubyte)iks_clamp8(a);
    glColor4ub(iks_cr, iks_cg, iks_cb, iks_ca);
}

static void iks_gl_begin(int64_t mode) {
    GLenum m;
    switch (mode) {
        case 0: m = GL_POINTS; break;
        case 1: m = GL_LINES; break;
        case 2: m = GL_LINE_STRIP; break;
        case 4: m = GL_TRIANGLE_STRIP; break;
        case 5: m = GL_TRIANGLE_FAN; break;
        case 6: m = GL_QUADS; break;
        case 7: m = GL_QUAD_STRIP; break;
        default: m = GL_TRIANGLES; break;
    }
    iks_draw_mode = (int)m;
    iks_vn = 0;
    iks_in_begin = 1;
}

static void iks_gl_vertex(double x, double y, double z) {
    if (!iks_in_begin) return;
    if (iks_vn >= iks_vcap) {
        int nc = iks_vcap ? iks_vcap * 2 : 4096;
        IksV* nb = (IksV*)realloc(iks_vbuf, (size_t)nc * sizeof(IksV));
        if (!nb) {
            fprintf(stderr, "out of memory\n");
            exit(1);
        }
        iks_vbuf = nb;
        iks_vcap = nc;
    }
    IksV* v = &iks_vbuf[iks_vn++];
    v->x = (float)x;
    v->y = (float)y;
    v->z = (float)z;
    v->r = iks_cr;
    v->g = iks_cg;
    v->b = iks_cb;
    v->a = iks_ca;
}

static void iks_gl_end(void) {
    if (!iks_in_begin) return;
    iks_in_begin = 0;
    if (iks_vn <= 0) return;
    glEnableClientState(GL_VERTEX_ARRAY);
    glEnableClientState(GL_COLOR_ARRAY);
    glVertexPointer(3, GL_FLOAT, sizeof(IksV), &iks_vbuf[0].x);
    glColorPointer(4, GL_UNSIGNED_BYTE, sizeof(IksV), &iks_vbuf[0].r);
    glDrawArrays((GLenum)iks_draw_mode, 0, iks_vn);
    glDisableClientState(GL_COLOR_ARRAY);
    glDisableClientState(GL_VERTEX_ARRAY);
    iks_vn = 0;
}

static void iks_gl_rect(double x, double y, double w, double h) {
    iks_gl_begin(6);
    iks_gl_vertex(x, y, 0); iks_gl_vertex(x + w, y, 0);
    iks_gl_vertex(x + w, y + h, 0); iks_gl_vertex(x, y + h, 0);
    iks_gl_end();
}

static void iks_gl_circle(double cx, double cy, double r, int64_t segs) {
    if (segs < 3) segs = 32;
    iks_gl_begin(5);
    iks_gl_vertex(cx, cy, 0);
    double step = 6.283185307179586 / (double)segs;
    for (int64_t i = 0; i <= segs; i++) {
        double a = (double)i * step;
        iks_gl_vertex(cx + cos(a) * r, cy + sin(a) * r, 0);
    }
    iks_gl_end();
}

static void iks_gl_line(double x1, double y1, double x2, double y2) {
    iks_gl_begin(1);
    iks_gl_vertex(x1, y1, 0);
    iks_gl_vertex(x2, y2, 0);
    iks_gl_end();
}

static void iks_gl_point_size(double s) { glPointSize((GLfloat)s); }
static void iks_gl_line_width(double w) { glLineWidth((GLfloat)w); }

static void iks_gl_depth(int64_t on) {
    if (on) glEnable(GL_DEPTH_TEST);
    else glDisable(GL_DEPTH_TEST);
}

static void iks_gl_blend(int64_t on) {
    if (on) {
        glEnable(GL_BLEND);
        glBlendFunc(GL_ONE, GL_ONE);
    } else {
        glDisable(GL_BLEND);
    }
}

static void iks_gl_proj_ortho(double l, double r, double b, double t) {
    glMatrixMode(GL_PROJECTION);
    glLoadIdentity();
    glOrtho(l, r, b, t, -1.0, 1.0);
    glMatrixMode(GL_MODELVIEW);
}

static void iks_gl_title(const char* t) {
    if (!iks_dpy || !iks_win) return;
    XStoreName(iks_dpy, iks_win, t ? t : "IKS");
    XFlush(iks_dpy);
}

static void iks_gl_fullscreen(int64_t on) {
    if (!iks_dpy || !iks_win) return;
    Atom wm_state = XInternAtom(iks_dpy, "_NET_WM_STATE", False);
    Atom fs = XInternAtom(iks_dpy, "_NET_WM_STATE_FULLSCREEN", False);
    XEvent ev;
    memset(&ev, 0, sizeof(ev));
    ev.xclient.type = ClientMessage;
    ev.xclient.window = iks_win;
    ev.xclient.message_type = wm_state;
    ev.xclient.format = 32;
    ev.xclient.data.l[0] = on ? 1 : 0;
    ev.xclient.data.l[1] = (long)fs;
    ev.xclient.data.l[2] = 0;
    ev.xclient.data.l[3] = 1;
    XSendEvent(iks_dpy, RootWindow(iks_dpy, DefaultScreen(iks_dpy)), False,
               SubstructureRedirectMask | SubstructureNotifyMask, &ev);
    XFlush(iks_dpy);
}

static int64_t iks_gl_width(void) {
    XWindowAttributes wa;
    if (iks_dpy && iks_win && XGetWindowAttributes(iks_dpy, iks_win, &wa)) return wa.width;
    return iks_gw;
}

static int64_t iks_gl_height(void) {
    XWindowAttributes wa;
    if (iks_dpy && iks_win && XGetWindowAttributes(iks_dpy, iks_win, &wa)) return wa.height;
    return iks_gh;
}

static int64_t iks_gl_swap(void) {
    if (!iks_glctx) return 0;
    glXSwapBuffers(iks_dpy, iks_win);
    return iks_gfx_poll();
}

static void iks_d3d_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d11 error: only available in Windows builds\n"); exit(1); }
static void iks_d3d_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d3d_draw(IksArray a) { (void)a; }
static int64_t iks_d3d_present(void) { return 0; }
static void iks_d3d_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d3d_begin(int64_t m) { (void)m; }
static void iks_d3d_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d3d_end(void) {}
static void iks_d3d_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d3d_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d3d_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static void iks_d3d_title(const char* t) { (void)t; }
static int64_t iks_d3d_width(void) { return 0; }
static int64_t iks_d3d_height(void) { return 0; }
static void iks_d12_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d12 error: only available in Windows builds\n"); exit(1); }
static void iks_d12_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d12_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d12_begin(int64_t m) { (void)m; }
static void iks_d12_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d12_end(void) {}
static void iks_d12_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d12_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d12_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_d12_swap(void) { return 0; }
static void iks_d12_title(const char* t) { (void)t; }
static int64_t iks_d12_width(void) { return 0; }
static int64_t iks_d12_height(void) { return 0; }
static void iks_d9_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d9 error: only available in Windows builds\n"); exit(1); }
static void iks_d9_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d9_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d9_begin(int64_t m) { (void)m; }
static void iks_d9_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d9_end(void) {}
static void iks_d9_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d9_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d9_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_d9_swap(void) { return 0; }
static void iks_d9_title(const char* t) { (void)t; }
static int64_t iks_d9_width(void) { return 0; }
static int64_t iks_d9_height(void) { return 0; }
static void iks_d10_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d10 error: only available in Windows builds\n"); exit(1); }
static void iks_d10_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d10_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d10_begin(int64_t m) { (void)m; }
static void iks_d10_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d10_end(void) {}
static void iks_d10_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d10_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d10_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_d10_swap(void) { return 0; }
static void iks_d10_title(const char* t) { (void)t; }
static int64_t iks_d10_width(void) { return 0; }
static int64_t iks_d10_height(void) { return 0; }
static void iks_vk_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "vulkan error: only available in Windows builds\n"); exit(1); }
static void iks_vk_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_vk_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_vk_begin(int64_t m) { (void)m; }
static void iks_vk_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_vk_end(void) {}
static void iks_vk_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_vk_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_vk_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_vk_swap(void) { return 0; }
static void iks_vk_title(const char* t) { (void)t; }
static int64_t iks_vk_width(void) { return 0; }
static int64_t iks_vk_height(void) { return 0; }

#else

static void iks_gfx_open(int64_t w, int64_t h, const char* title) {
    (void)w; (void)h; (void)title;
    fprintf(stderr, "gfx error: graphics is only available in Windows builds\n");
    exit(1);
}
static int64_t iks_gfx_poll(void) { return 0; }
static void iks_gfx_present(void) {}
static void iks_gfx_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_gfx_pixel(int64_t x, int64_t y, int64_t r, int64_t g, int64_t b) {
    (void)x; (void)y; (void)r; (void)g; (void)b;
}
static void iks_gfx_add(int64_t x, int64_t y, int64_t r, int64_t g, int64_t b) {
    (void)x; (void)y; (void)r; (void)g; (void)b;
}
static void iks_gfx_line(int64_t a, int64_t b, int64_t c, int64_t d, int64_t e, int64_t f, int64_t g) {
    (void)a; (void)b; (void)c; (void)d; (void)e; (void)f; (void)g;
}
static int64_t iks_gfx_width(void) { return 0; }
static int64_t iks_gfx_height(void) { return 0; }
static double iks_gfx_time(void) { return 0.0; }
static int64_t iks_gfx_mouse_x(void) { return 0; }
static int64_t iks_gfx_mouse_y(void) { return 0; }
static int64_t iks_gfx_key_down(int64_t vk) { (void)vk; return 0; }
static void iks_sleep(int64_t ms) { (void)ms; }
static void iks_gl_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "gfx error: OpenGL is only available in Windows builds\n"); exit(1); }
static void iks_gl_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_gl_view(int64_t x, int64_t y, int64_t w, int64_t h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_gl_proj(double a, double b, double c, double d) { (void)a; (void)b; (void)c; (void)d; }
static void iks_gl_push(void) {}
static void iks_gl_pop(void) {}
static void iks_gl_translate(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_gl_rotate(double a, double x, double y, double z) { (void)a; (void)x; (void)y; (void)z; }
static void iks_gl_scale(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_gl_color(int64_t r, int64_t g, int64_t b, int64_t a) { (void)r; (void)g; (void)b; (void)a; }
static void iks_gl_begin(int64_t m) { (void)m; }
static void iks_gl_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_gl_end(void) {}
static double iks_gpu_load(void) { return 0.0; }
static double iks_cpu_load(void) { return 0.0; }
static int64_t iks_npu_available(void) { return 0; }
static int64_t iks_npu_count(void) { return 0; }
static char* iks_npu_name(void) { return ""; }
static double iks_npu_tops(void) { return 0.0; }
static double iks_npu_load(void) { return 0.0; }
static void iks_gl_title(const char* t) { (void)t; }
static void iks_gl_fullscreen(int64_t on) { (void)on; }
static int64_t iks_gl_width(void) { return 0; }
static int64_t iks_gl_height(void) { return 0; }
static void iks_gl_proj_ortho(double l, double r, double b, double t) { (void)l; (void)r; (void)b; (void)t; }
static void iks_gl_point_size(double s) { (void)s; }
static void iks_gl_line_width(double w) { (void)w; }
static void iks_gl_depth(int64_t on) { (void)on; }
static void iks_gl_blend(int64_t on) { (void)on; }
static int64_t iks_gl_swap(void) { return 0; }
static int64_t iks_gfx_mouse_button(int64_t b) { (void)b; return 0; }
static void iks_gl_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_gl_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_gl_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static void iks_d3d_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d11 error: only available in Windows builds\n"); exit(1); }
static void iks_d3d_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d3d_draw(IksArray a) { (void)a; }
static int64_t iks_d3d_present(void) { return 0; }
static void iks_d3d_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d3d_begin(int64_t m) { (void)m; }
static void iks_d3d_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d3d_end(void) {}
static void iks_d3d_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d3d_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d3d_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static void iks_d3d_title(const char* t) { (void)t; }
static int64_t iks_d3d_width(void) { return 0; }
static int64_t iks_d3d_height(void) { return 0; }
static void iks_d12_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d12 error: only available in Windows builds\n"); exit(1); }
static void iks_d12_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d12_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d12_begin(int64_t m) { (void)m; }
static void iks_d12_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d12_end(void) {}
static void iks_d12_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d12_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d12_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_d12_swap(void) { return 0; }
static void iks_d12_title(const char* t) { (void)t; }
static int64_t iks_d12_width(void) { return 0; }
static int64_t iks_d12_height(void) { return 0; }
static void iks_d9_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d9 error: only available in Windows builds\n"); exit(1); }
static void iks_d9_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d9_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d9_begin(int64_t m) { (void)m; }
static void iks_d9_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d9_end(void) {}
static void iks_d9_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d9_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d9_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_d9_swap(void) { return 0; }
static void iks_d9_title(const char* t) { (void)t; }
static int64_t iks_d9_width(void) { return 0; }
static int64_t iks_d9_height(void) { return 0; }
static void iks_d10_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "d3d10 error: only available in Windows builds\n"); exit(1); }
static void iks_d10_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d10_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_d10_begin(int64_t m) { (void)m; }
static void iks_d10_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_d10_end(void) {}
static void iks_d10_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_d10_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_d10_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_d10_swap(void) { return 0; }
static void iks_d10_title(const char* t) { (void)t; }
static int64_t iks_d10_width(void) { return 0; }
static int64_t iks_d10_height(void) { return 0; }
static void iks_vk_init(int64_t w, int64_t h, const char* title) { (void)w; (void)h; (void)title; fprintf(stderr, "vulkan error: only available in Windows builds\n"); exit(1); }
static void iks_vk_clear(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_vk_color(int64_t r, int64_t g, int64_t b) { (void)r; (void)g; (void)b; }
static void iks_vk_begin(int64_t m) { (void)m; }
static void iks_vk_vertex(double x, double y, double z) { (void)x; (void)y; (void)z; }
static void iks_vk_end(void) {}
static void iks_vk_rect(double x, double y, double w, double h) { (void)x; (void)y; (void)w; (void)h; }
static void iks_vk_circle(double cx, double cy, double r, int64_t segs) { (void)cx; (void)cy; (void)r; (void)segs; }
static void iks_vk_line(double x1, double y1, double x2, double y2) { (void)x1; (void)y1; (void)x2; (void)y2; }
static int64_t iks_vk_swap(void) { return 0; }
static void iks_vk_title(const char* t) { (void)t; }
static int64_t iks_vk_width(void) { return 0; }
static int64_t iks_vk_height(void) { return 0; }

#endif
"##;

const RESERVED: &[&str] = &[
    "print", "len", "str", "int", "float", "input", "push", "rand", "exit", "sleep", "assert", "beep",
    "sin", "cos", "sqrt", "abs", "floor", "pow",
    "gfx_open", "gfx_poll", "gfx_present", "gfx_clear", "gfx_pixel", "gfx_add", "gfx_line",
    "gfx_width", "gfx_height", "gfx_time", "gfx_mouse_x", "gfx_mouse_y", "gfx_key_down",
    "gfx_mouse_button", "touch_x", "touch_y",
    "gl_init", "gl_clear", "gl_view", "gl_proj", "gl_push", "gl_pop", "gl_translate", "gl_rotate",
    "gl_scale", "gl_color", "gl_begin", "gl_vertex", "gl_end", "gl_point_size", "gl_line_width",
    "gl_depth", "gl_blend", "gl_swap", "gl_title", "gl_fullscreen", "gl_width", "gl_height",
    "gl_proj_ortho", "cpu_load", "gpu_load",
    "npu_available", "npu_name", "npu_count", "npu_tops", "npu_load",
    "sprite_load", "sprite_draw", "sprite_width", "sprite_height", "sprite_unload",
    "d3d_init", "d3d_clear", "d3d_draw", "d3d_present",
    "d3d_color", "d3d_begin", "d3d_vertex", "d3d_end",
    "d3d_title", "d3d_width", "d3d_height",
    "min", "max", "clamp", "lerp", "atan2", "round", "ceil", "sign", "hypot",
    "vec2", "vec3", "vec4", "dot", "cross", "length", "normalize", "distance", "rotate",
    "rand_range", "rand_int",
    "char_at", "substr", "str_find", "str_replace", "str_upper", "str_lower", "str_trim",
    "str_starts", "str_ends", "str_cmp", "str_split", "str_join",
    "pop", "arr_remove", "arr_insert", "arr_slice", "arr_reverse", "arr_sort",
    "arr_index_of", "arr_contains",
    "file_read", "file_write", "file_exists", "file_delete",
    "map_new", "map_set", "map_get", "map_has", "map_remove", "map_keys", "map_len",
    "gl_rect", "gl_circle", "gl_line",
    "d3d_rect", "d3d_circle", "d3d_line",
    "dx12_init", "dx12_clear", "dx12_color", "dx12_begin", "dx12_vertex", "dx12_end", "dx12_swap",
    "dx12_title", "dx12_width", "dx12_height", "dx12_rect", "dx12_circle", "dx12_line",
    "d3d9_init", "d3d9_clear", "d3d9_color", "d3d9_begin", "d3d9_vertex", "d3d9_end", "d3d9_swap",
    "d3d9_title", "d3d9_width", "d3d9_height", "d3d9_rect", "d3d9_circle", "d3d9_line",
    "dx10_init", "dx10_clear", "dx10_color", "dx10_begin", "dx10_vertex", "dx10_end", "dx10_swap",
    "dx10_title", "dx10_width", "dx10_height", "dx10_rect", "dx10_circle", "dx10_line",
    "vk_init", "vk_clear", "vk_color", "vk_begin", "vk_vertex", "vk_end", "vk_swap",
    "vk_title", "vk_width", "vk_height", "vk_rect", "vk_circle", "vk_line",
    "webgpu_init", "webgpu_clear", "webgpu_color", "webgpu_begin", "webgpu_vertex", "webgpu_end",
    "webgpu_swap", "webgpu_title", "webgpu_width", "webgpu_height",
    "webgpu_rect", "webgpu_circle", "webgpu_line",
];

#[derive(Clone, Copy, PartialEq)]
pub enum BuildMode {
    Normal,
    Test,
}

#[derive(Clone)]
struct FnSig {
    params: Vec<Ty>,
    ret: Ty,
    span: Span,
    is_extern: bool,
}

#[derive(Clone)]
struct StructInfo {
    fields: Vec<(String, Ty)>,
}

/// Per-enum metadata: declaration span plus every variant's name and payload
/// type list, indexed by declaration order (the runtime tag).
#[derive(Clone)]
struct EnumInfo {
    span: Span,
    names: Vec<String>,
    params: Vec<Vec<Ty>>,
}

/// Global constructor table: `Circle` -> which enum it belongs to and how to
/// build it. Variant names are unique across the whole program.
#[derive(Clone)]
struct VariantInfo {
    enum_name: String,
    index: usize,
    params: Vec<Ty>,
    span: Span,
}

#[derive(Clone)]
struct VarInfo {
    ty: Ty,
    mutable: bool,
    used: bool,
    span: Span,
}

struct Cg {
    out: String,
    sigs: HashMap<String, FnSig>,
    structs: HashMap<String, StructInfo>,
    enums: HashMap<String, EnumInfo>,
    variants: HashMap<String, VariantInfo>,
    scopes: Vec<HashMap<String, VarInfo>>,
    ret: Ty,
    loops: usize,
    switch_seq: usize,
    is_main: bool,
    tuple_defs: String,
    tuple_seen: HashSet<String>,
    warnings: Vec<Diag>,
    errors: Vec<Diag>,
    sprite_used: bool,
    /// Spans of enum switches that covered every variant without a `default`.
    /// Such a switch is a definite terminator, exactly like one with `default`.
    exhaustive_switches: Vec<Span>,
}

// Sprite backend: stb_image loader + GL 1.1 textured quads.
// Emitted only when a sprite_* builtin is used. Self-guarded per platform so the
// non-Windows branch compiles to harmless stubs.
const SPRITE_C: &str = r####"
#if defined(IKS_WIN)
#define STB_IMAGE_IMPLEMENTATION
#include "stb_image.h"

#define IKS_MAX_SPRITES 512
typedef struct { unsigned int tex; int w, h; int live; } IksSprite;
static IksSprite iks_sprites[IKS_MAX_SPRITES];
static int iks_sprite_n = 0;

static int64_t iks_sprite_load(const char* path) {
    if (!iks_glrc) {
        fprintf(stderr, "sprite error: call gl_init() before sprite_load()\n");
        return -1;
    }
    if (iks_sprite_n >= IKS_MAX_SPRITES) return -1;
    int w = 0, h = 0, n = 0;
    unsigned char* data = stbi_load(path, &w, &h, &n, 4);
    if (!data) {
        fprintf(stderr, "sprite error: cannot load '%s'\n", path);
        return -1;
    }
    unsigned int tex = 0;
    glGenTextures(1, &tex);
    glBindTexture(GL_TEXTURE_2D, tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE, data);
    stbi_image_free(data);
    int idx = iks_sprite_n++;
    iks_sprites[idx].tex = tex;
    iks_sprites[idx].w = w;
    iks_sprites[idx].h = h;
    iks_sprites[idx].live = 1;
    return (int64_t)idx;
}

static void iks_sprite_draw(int64_t id, double x, double y, double w, double h) {
    if (id < 0 || id >= iks_sprite_n || !iks_sprites[id].live) return;
    glEnable(GL_TEXTURE_2D);
    glBindTexture(GL_TEXTURE_2D, iks_sprites[id].tex);
    glColor4ub(iks_cr, iks_cg, iks_cb, iks_ca);
    glBegin(GL_QUADS);
    glTexCoord2f(0.0f, 0.0f); glVertex2d(x, y);
    glTexCoord2f(1.0f, 0.0f); glVertex2d(x + w, y);
    glTexCoord2f(1.0f, 1.0f); glVertex2d(x + w, y + h);
    glTexCoord2f(0.0f, 1.0f); glVertex2d(x, y + h);
    glEnd();
    glDisable(GL_TEXTURE_2D);
}

static int64_t iks_sprite_width(int64_t id) {
    if (id < 0 || id >= iks_sprite_n || !iks_sprites[id].live) return 0;
    return iks_sprites[id].w;
}

static int64_t iks_sprite_height(int64_t id) {
    if (id < 0 || id >= iks_sprite_n || !iks_sprites[id].live) return 0;
    return iks_sprites[id].h;
}

static void iks_sprite_unload(int64_t id) {
    if (id < 0 || id >= iks_sprite_n || !iks_sprites[id].live) return;
    unsigned int tex = iks_sprites[id].tex;
    glDeleteTextures(1, &tex);
    iks_sprites[id].live = 0;
}
#elif defined(IKS_ANDROID)
#define STB_IMAGE_IMPLEMENTATION
#include "stb_image.h"

#define IKS_MAX_SPRITES 512
typedef struct { unsigned int tex; int w, h; int live; } IksSprite;
static IksSprite iks_sprites[IKS_MAX_SPRITES];
static int iks_sprite_n = 0;

static int64_t iks_sprite_load(const char* path) {
    if (!iks_glstate_ok) {
        IKERR("sprite error: call gl_init() before sprite_load()");
        return -1;
    }
    if (iks_sprite_n >= IKS_MAX_SPRITES) return -1;
    int w = 0, h = 0, n = 0;
    unsigned char* data = stbi_load(path, &w, &h, &n, 4);
    if (!data) {
        IKERR("sprite error: cannot load '%s'", path);
        return -1;
    }
    unsigned int tex = 0;
    glGenTextures(1, &tex);
    glBindTexture(GL_TEXTURE_2D, tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE, data);
    stbi_image_free(data);
    int idx = iks_sprite_n++;
    iks_sprites[idx].tex = tex;
    iks_sprites[idx].w = w;
    iks_sprites[idx].h = h;
    iks_sprites[idx].live = 1;
    return (int64_t)idx;
}

static void iks_sprite_draw(int64_t id, double x, double y, double w, double h) {
    if (id < 0 || id >= iks_sprite_n || !iks_sprites[id].live) return;
    GLuint prev = iks_cur_tex;
    iks_cur_tex = iks_sprites[id].tex;
    iks_gl_begin(6);
    iks_gl_vertex_uv(x, y, 0, 0.0f, 0.0f);
    iks_gl_vertex_uv(x + w, y, 0, 1.0f, 0.0f);
    iks_gl_vertex_uv(x + w, y + h, 0, 1.0f, 1.0f);
    iks_gl_vertex_uv(x, y + h, 0, 0.0f, 1.0f);
    iks_gl_end();
    iks_cur_tex = prev;
}

static int64_t iks_sprite_width(int64_t id) {
    if (id < 0 || id >= iks_sprite_n || !iks_sprites[id].live) return 0;
    return iks_sprites[id].w;
}

static int64_t iks_sprite_height(int64_t id) {
    if (id < 0 || id >= iks_sprite_n || !iks_sprites[id].live) return 0;
    return iks_sprites[id].h;
}

static void iks_sprite_unload(int64_t id) {
    if (id < 0 || id >= iks_sprite_n || !iks_sprites[id].live) return;
    unsigned int tex = iks_sprites[id].tex;
    glDeleteTextures(1, &tex);
    iks_sprites[id].live = 0;
}
#else
static int64_t iks_sprite_load(const char* path) { (void)path; return -1; }
static void iks_sprite_draw(int64_t id, double x, double y, double w, double h) {
    (void)id; (void)x; (void)y; (void)w; (void)h;
}
static int64_t iks_sprite_width(int64_t id) { (void)id; return 0; }
static int64_t iks_sprite_height(int64_t id) { (void)id; return 0; }
static void iks_sprite_unload(int64_t id) { (void)id; }
#endif
"####;

pub fn generate(program: &Program, mode: BuildMode) -> Result<(String, Vec<Diag>, Vec<Diag>), Vec<Diag>> {
    let mut sigs = HashMap::new();
    for f in &program.funcs {
        if RESERVED.contains(&f.name.as_str()) {
            return Err(vec![Diag::new(
                f.span,
                format!("'{}' is a built-in name and cannot be used as a function name", f.name),
            )]);
        }
        if let Some(prev) = sigs.get(&f.name) {
            let prev: &FnSig = prev;
            return Err(vec![Diag::new(
                f.span,
                format!(
                    "function '{}' is already defined (first definition is on line {})",
                    f.name, prev.span.line
                ),
            )]);
        }
        let sig = FnSig {
            params: f.params.iter().map(|p| p.ty.clone()).collect(),
            ret: f.ret.clone(),
            span: f.span,
            is_extern: f.is_extern,
        };
        sigs.insert(f.name.clone(), sig);
    }

    let mut structs: HashMap<String, StructInfo> = HashMap::new();
    for s in &program.structs {
        if structs.contains_key(&s.name) {
            return Err(vec![Diag::new(s.span, format!("struct '{}' is already defined", s.name))]);
        }
        structs.insert(
            s.name.clone(),
            StructInfo { fields: s.fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect() },
        );
    }
    let mut enums: HashMap<String, EnumInfo> = HashMap::new();
    let mut variants: HashMap<String, VariantInfo> = HashMap::new();
    let mut setup_errors: Vec<Diag> = Vec::new();
    for e in &program.enums {
        if structs.contains_key(&e.name) {
            setup_errors.push(Diag::new(e.span, format!("type '{}' is already defined as a struct", e.name)));
            continue;
        }
        if let Some(prev) = enums.get(&e.name) {
            setup_errors.push(Diag::new(
                e.span,
                format!("enum '{}' is already defined (first definition is on line {})", e.name, prev.span.line),
            ));
            continue;
        }
        enums.insert(
            e.name.clone(),
            EnumInfo {
                span: e.span,
                names: e.variants.iter().map(|v| v.name.clone()).collect(),
                params: e.variants.iter().map(|v| v.params.clone()).collect(),
            },
        );
    }
    if !setup_errors.is_empty() {
        return Err(setup_errors);
    }
    for e in &program.enums {
        for (i, v) in e.variants.iter().enumerate() {
            if RESERVED.contains(&v.name.as_str()) {
                return Err(vec![Diag::new(v.span, format!("variant '{}' is a built-in name", v.name))]);
            }
            if let Some(prev) = variants.get(&v.name) {
                return Err(vec![Diag::new(
                    v.span,
                    format!(
                        "variant '{}' is already defined (first defined in enum '{}' on line {})",
                        v.name, prev.enum_name, prev.span.line
                    ),
                )]);
            }
            if sigs.contains_key(&v.name) {
                return Err(vec![Diag::new(
                    v.span,
                    format!("variant '{}' conflicts with the function of the same name", v.name),
                )]);
            }
            variants.insert(
                v.name.clone(),
                VariantInfo { enum_name: e.name.clone(), index: i, params: v.params.clone(), span: v.span },
            );
        }
    }

    for s in &program.structs {
        for f in &s.fields {
            if let Err(e) = check_ty_resolvable(&f.ty, &structs, &enums, f.span) {
                setup_errors.push(e);
            }
        }
    }
    for e in &program.enums {
        for v in &e.variants {
            for t in &v.params {
                if let Err(err) = check_ty_resolvable(t, &structs, &enums, v.span) {
                    setup_errors.push(err);
                }
            }
        }
    }
    if !setup_errors.is_empty() {
        return Err(setup_errors);
    }

    if mode == BuildMode::Normal {
        let Some(main_fn) = program.funcs.iter().find(|f| f.name == "main") else {
            return Err(vec![Diag::new(
                Span::new(0, 1, 1),
                "program has no 'main' function; add: fn main() -> int { return 0; }",
            )]);
        };
        if main_fn.is_extern {
            return Err(vec![Diag::new(main_fn.span, "'main' cannot be extern")]);
        }
        if !main_fn.params.is_empty() {
            return Err(vec![Diag::new(main_fn.span, "'main' function must not take parameters")]);
        }
        if !matches!(main_fn.ret, Ty::Int | Ty::Void) {
            return Err(vec![Diag::new(main_fn.span, "'main' must return 'int' or nothing")]);
        }
    }

    for f in &program.funcs {
        for p in &f.params {
            check_ty_resolvable(&p.ty, &structs, &enums, p.span).map_err(|e| vec![e])?;
        }
        check_ty_resolvable(&f.ret, &structs, &enums, f.span).map_err(|e| vec![e])?;
    }

    let order = type_emit_order(&program.structs, &program.enums).map_err(|e| vec![e])?;

    let mut cg = Cg {
        out: String::new(),
        sigs,
        structs,
        enums,
        variants,
        scopes: Vec::new(),
        ret: Ty::Void,
        loops: 0,
        switch_seq: 0,
        is_main: false,
        tuple_defs: String::new(),
        tuple_seen: HashSet::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
        sprite_used: false,
        exhaustive_switches: Vec::new(),
    };

    cg.out.push_str(PRELUDE);
    cg.out.push('\n');

    for name in &order {
        if let Some(e) = program.enums.iter().find(|x| &x.name == name) {
            cg.out.push_str(&format!("typedef struct S_{} {{\n", e.name));
            cg.out.push_str("    int64_t tag;\n");
            cg.out.push_str("    union {\n        int64_t _none;\n");
            for (i, v) in e.variants.iter().enumerate() {
                if v.params.is_empty() {
                    continue;
                }
                cg.out.push_str("        struct {\n");
                for (j, t) in v.params.iter().enumerate() {
                    cg.out.push_str(&format!("            {} _{};\n", c_ty(t), j));
                }
                cg.out.push_str(&format!("        }} _v{};\n", i));
            }
            cg.out.push_str("    } u;\n");
            cg.out.push_str(&format!("}} S_{};\n\n", e.name));
            continue;
        }
        let Some(decl) = program.structs.iter().find(|s| &s.name == name) else {
            continue;
        };
        cg.out.push_str(&format!("typedef struct S_{} {{\n", decl.name));
        for field in &decl.fields {
            cg.out
                .push_str(&format!("    {} {};\n", c_ty(&field.ty), mangle_field(&field.name)));
        }
        cg.out.push_str(&format!("}} S_{};\n\n", decl.name));
    }

    let mut protos = String::new();
    for f in &program.funcs {
        protos.push_str(&cg.proto_for(f));
        protos.push('\n');
    }

    let mut test_errors: Vec<Diag> = Vec::new();
    let tests: Vec<&FnDecl> = if mode == BuildMode::Test {
        let mut list = Vec::new();
        for f in &program.funcs {
            if f.name.starts_with("test_") {
                if f.is_extern {
                    test_errors.push(Diag::new(f.span, "test functions cannot be extern"));
                    continue;
                }
                if !f.params.is_empty() {
                    test_errors.push(Diag::new(f.span, "test functions must not take parameters"));
                    continue;
                }
                list.push(f);
            }
        }
        list
    } else {
        Vec::new()
    };
    if !test_errors.is_empty() {
        return Err(test_errors);
    }

    let mut bodies = String::new();
    for f in &program.funcs {
        if f.is_extern {
            continue;
        }
        if mode == BuildMode::Test && f.name == "main" {
            continue;
        }
        bodies.push('\n');
        let saved = std::mem::take(&mut cg.out);
        cg.out.push('\n');
        if let Err(e) = cg.emit_fn(f) {
            cg.errors.push(e);
        }
        bodies.push_str(&cg.out);
        cg.out = saved;
    }

    if cg.sprite_used {
        cg.out.push_str(SPRITE_C);
        cg.out.push('\n');
    }
    cg.out.push_str(&cg.tuple_defs.clone());
    cg.out.push_str(&protos);
    cg.out.push_str(&bodies);

    if mode == BuildMode::Test {
        cg.emit_test_harness(&tests);
    }

    let warnings = std::mem::take(&mut cg.warnings);
    let errors = std::mem::take(&mut cg.errors);
    Ok((cg.out, errors, warnings))
}

fn mangle_field(name: &str) -> String {
    format!("f_{}", name)
}

fn c_name(name: &str, is_extern: bool) -> String {
    if is_extern {
        name.to_string()
    } else if name == "main" {
        "main".to_string()
    } else {
        format!("f_{}", name)
    }
}

fn c_var(name: &str) -> String {
    format!("v_{}", name)
}

fn c_ty(t: &Ty) -> String {
    match t {
        Ty::Int => "int64_t".to_string(),
        Ty::Float => "double".to_string(),
        Ty::Bool => "bool".to_string(),
        Ty::Str => "char*".to_string(),
        Ty::Void => "void".to_string(),
        Ty::Ptr => "void*".to_string(),
        Ty::Array(_) => "IksArray".to_string(),
        Ty::Named(n) => format!("S_{}", n),
        Ty::HashMap => "IksMap*".to_string(),
        Ty::Vec2 => "IksVec2".to_string(),
        Ty::Vec3 => "IksVec3".to_string(),
        Ty::Vec4 => "IksVec4".to_string(),
        Ty::Tuple(ts) => {
            let parts: Vec<String> = ts.iter().map(|t| c_ty(t).replace(['*', ' '], "_")).collect();
            format!("T_{}", parts.join("_"))
        }
    }
}

fn tag_of(t: &Ty) -> u8 {
    match t {
        Ty::Int => 0,
        Ty::Float => 1,
        Ty::Bool => 2,
        Ty::Str => 3,
        Ty::Array(_) => 4,
        _ => 5,
    }
}

fn check_ty_resolvable(
    t: &Ty,
    structs: &HashMap<String, StructInfo>,
    enums: &HashMap<String, EnumInfo>,
    span: Span,
) -> Result<(), Diag> {
    match t {
        Ty::Named(n) => {
            if !structs.contains_key(n) && !enums.contains_key(n) {
                return Err(Diag::new(span, format!("unknown type '{}'", n)));
            }
            Ok(())
        }
        Ty::Array(e) => check_ty_resolvable(e, structs, enums, span),
        _ => Ok(()),
    }
}

fn type_deps(t: &Ty) -> Vec<String> {
    match t {
        Ty::Named(n) => vec![n.clone()],
        Ty::Array(e) => type_deps(e),
        _ => vec![],
    }
}

/// Emit structs and enums in dependency order so every named type is a
/// complete C type where it is first used.
fn type_emit_order(structs: &[StructDecl], enums: &[EnumDecl]) -> Result<Vec<String>, Diag> {
    let mut done: HashSet<String> = HashSet::new();
    let mut order = Vec::new();
    for s in structs {
        visit_type(&s.name, structs, enums, &mut done, &mut order, &mut HashSet::new())?;
    }
    for e in enums {
        visit_type(&e.name, structs, enums, &mut done, &mut order, &mut HashSet::new())?;
    }
    Ok(order)
}

fn visit_type(
    name: &str,
    structs: &[StructDecl],
    enums: &[EnumDecl],
    done: &mut HashSet<String>,
    order: &mut Vec<String>,
    stack: &mut HashSet<String>,
) -> Result<(), Diag> {
    if done.contains(name) {
        return Ok(());
    }
    if !stack.insert(name.to_string()) {
        let kind = if enums.iter().any(|e| e.name == name) { "enum" } else { "struct" };
        let span = enums
            .iter().find(|e| e.name == name).map(|e| e.span)
            .or_else(|| structs.iter().find(|s| s.name == name).map(|s| s.span))
            .unwrap_or(Span::new(0, 1, 1));
        return Err(Diag::new(
            span,
            format!("recursive {} definition involving '{}'; use 'ptr' to break the cycle", kind, name),
        ));
    }
    let deps: Vec<String> = if let Some(s) = structs.iter().find(|x| x.name == name) {
        s.fields.iter().flat_map(|f| type_deps(&f.ty)).collect()
    } else if let Some(e) = enums.iter().find(|x| x.name == name) {
        e.variants.iter().flat_map(|v| v.params.iter().flat_map(type_deps)).collect()
    } else {
        Vec::new()
    };
    for d in deps {
        let known = structs.iter().any(|x| x.name == d) || enums.iter().any(|x| x.name == d);
        if known {
            visit_type(&d, structs, enums, done, order, stack)?;
        }
    }
    stack.remove(name);
    done.insert(name.to_string());
    order.push(name.to_string());
    Ok(())
}

impl Cg {
    fn proto_for(&self, f: &FnDecl) -> String {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("{} {}", c_ty(&p.ty), c_var(&p.name)))
            .collect();
        if f.name == "main" && !f.is_extern {
            return "int main(void);".to_string();
        }
        let prefix = if f.is_extern { "extern " } else { "" };
        format!(
            "{}{} {}({});",
            prefix,
            c_ty(&f.ret),
            c_name(&f.name, f.is_extern),
            params.join(", ")
        )
    }

    fn emit_test_harness(&mut self, tests: &[&FnDecl]) {
        self.out.push_str("\nint main(void) {\n");
        self.out
            .push_str(&format!("    printf(\"running {} test(s)\\n\");\n", tests.len()));
        self.out.push_str("    int failed = 0;\n");
        for t in tests {
            self.out.push_str(&format!("    printf(\"{}: \");\n", t.name));
            if t.ret == Ty::Bool {
                self.out.push_str(&format!(
                    "    if (!{}()) {{ printf(\"FAILED\\n\"); failed = 1; }} else printf(\"ok\\n\");\n",
                    c_name(&t.name, false)
                ));
            } else {
                self.out.push_str(&format!("    {}();\n    printf(\"ok\\n\");\n", c_name(&t.name, false)));
            }
        }
        self.out
            .push_str("    if (failed) { printf(\"some tests FAILED\\n\"); return 1; }\n");
        self.out.push_str("    printf(\"all tests passed\\n\");\n    return 0;\n}\n");
    }

    /// Warn about any variables declared in the innermost scope that were never read.
    fn warn_unused_in_top_scope(&mut self) {
        if let Some(scope) = self.scopes.last() {
            let unused: Vec<(String, Span)> = scope
                .iter()
                .filter(|(_, v)| !v.used)
                .map(|(n, v)| (n.clone(), v.span))
                .collect();
            for (name, span) in unused {
                self.warnings
                    .push(Diag::warning(span, format!("unused variable '{}'", name)));
            }
        }
    }

    fn emit_fn(&mut self, f: &FnDecl) -> Result<(), Diag> {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("{} {}", c_ty(&p.ty), c_var(&p.name)))
            .collect();
        let c_ret = if f.name == "main" { "int".to_string() } else { c_ty(&f.ret) };
        self.out
            .push_str(&format!("{} {}({}) {{\n", c_ret, c_name(&f.name, false), params.join(", ")));

        self.scopes.push(HashMap::new());
        self.ret = f.ret.clone();
        self.is_main = f.name == "main";
        self.loops = 0;
        for p in &f.params {
            self.scopes
                .last_mut()
                .unwrap()
                .insert(p.name.clone(), VarInfo { ty: p.ty.clone(), mutable: true, used: true, span: p.span });
        }

        let mut terminated = false;
        let mut warned_unreachable = false;
        for stmt in &f.body.stmts {
            if terminated && !warned_unreachable {
                self.warnings.push(Diag::warning(
                    stmt.span(),
                    "unreachable code: this statement can never be executed",
                ));
                warned_unreachable = true;
            }
            if stmt_hard_terminates(stmt) {
                terminated = true;
            }
            let depth = self.scopes.len();
            if let Err(e) = self.emit_stmt(stmt) {
                self.errors.push(e);
                while self.scopes.len() > depth {
                    self.scopes.pop();
                }
            }
        }

        if f.name == "main" && f.ret == Ty::Void {
            self.out.push_str("    return 0;\n");
        } else if f.ret != Ty::Void && !block_terminates(&f.body, &self.exhaustive_switches) {
            return Err(Diag::new(
                f.span,
                format!(
                    "function '{}' must return a value of type '{}' on every execution path",
                    f.name, f.ret
                ),
            ));
        }

        self.warn_unused_in_top_scope();
        self.scopes.pop();
        self.out.push_str("}\n");
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diag> {
        match stmt {
            Stmt::Var { name, ty, init, mutable, span } => {
                let ann = ty.clone();
                let (mut code, mut ity) = self.expr_hint(init, ann.as_ref())?;
                if let Some(a) = ty {
                    code = self.coerce(&code, &ity, a, *span)?;
                    ity = a.clone();
                } else if ity == Ty::Void {
                    return Err(Diag::new(
                        *span,
                        format!(
                            "cannot declare '{}' without a type: initializer has type 'void'",
                            name
                        ),
                    ));
                }
                if self.scopes.last().unwrap().contains_key(name) {
                    return Err(Diag::new(
                        *span,
                        format!("variable '{}' is already defined in this scope", name),
                    ));
                }
                self.scopes.last_mut().unwrap().insert(
                    name.clone(),
                    VarInfo { ty: ity.clone(), mutable: *mutable, used: false, span: *span },
                );
                self.out
                    .push_str(&format!("    {} {} = {};\n", c_ty(&ity), c_var(name), code));
            }
            Stmt::Assign { target, value, span } => {
                self.emit_assign(target, value, *span)?;
            }
            Stmt::If { cond, then, els, .. } => {
                let (cc, ct) = self.expr(cond)?;
                let cc = self.cond_code(&cc, &ct, cond.span())?;
                self.out.push_str(&format!("    if ({}) {{\n", cc));
                self.emit_block_inner(then)?;
                self.out.push_str("    }");
                let mut cur = els.as_deref();
                while let Some(s) = cur {
                    match s {
                        Stmt::If { cond, then, els, .. } => {
                            let (cc, ct) = self.expr(cond)?;
                            let cc = self.cond_code(&cc, &ct, cond.span())?;
                            self.out.push_str(&format!(" else if ({}) {{\n", cc));
                            self.emit_block_inner(then)?;
                            self.out.push_str("    }");
                            cur = els.as_deref();
                        }
                        Stmt::Block(b) => {
                            self.out.push_str(" else {\n");
                            self.emit_block_inner(b)?;
                            self.out.push_str("    }");
                            cur = None;
                        }
                        other => {
                            return Err(Diag::new(other.span(), "invalid 'else' branch"));
                        }
                    }
                }
                self.out.push('\n');
            }
            Stmt::While { cond, body, .. } => {
                let (cc, ct) = self.expr(cond)?;
                let cc = self.cond_code(&cc, &ct, cond.span())?;
                self.out.push_str(&format!("    while ({}) {{\n", cc));
                self.loops += 1;
                self.emit_block_inner(body)?;
                self.loops -= 1;
                self.out.push_str("    }\n");
            }
            Stmt::Return { value, span } => {
                let ret_ty = self.ret.clone();
                match (value, &ret_ty) {
                    (Some(_), Ty::Void) => {
                        return Err(Diag::new(*span, "this function does not return a value"));
                    }
                    (None, Ty::Void) => {
                        if self.is_main {
                            self.out.push_str("    return 0;\n");
                        } else {
                            self.out.push_str("    return;\n");
                        }
                    }
                    (None, r) => {
                        return Err(Diag::new(
                            *span,
                            format!("expected a value of type '{}' in 'return'", r),
                        ));
                    }
                    (Some(e), r) => {
                        let (code, ty) = self.expr_hint(e, Some(r))?;
                        let code = self.coerce(&code, &ty, r, e.span())?;
                        self.out.push_str(&format!("    return {};\n", code));
                    }
                }
            }
            Stmt::Break(span) => {
                if self.loops == 0 {
                    return Err(Diag::new(*span, "'break' can only be used inside a 'while' loop"));
                }
                self.out.push_str("    break;\n");
            }
            Stmt::Continue(span) => {
                if self.loops == 0 {
                    return Err(Diag::new(*span, "'continue' can only be used inside a 'while' loop"));
                }
                self.out.push_str("    continue;\n");
            }
            Stmt::Block(b) => {
                self.emit_block_inner(b)?;
            }
            Stmt::Expr(e, _) => {
                self.emit_expr_stmt(e)?;
            }
            Stmt::Switch { value, cases, default, span } => {
                let (vc, vt) = self.expr(value)?;
                if let Ty::Named(n) = &vt {
                    if self.enums.contains_key(n) {
                        let n = n.clone();
                        self.emit_enum_switch(&n, &vc, cases, default.as_ref(), *span)?;
                        return Ok(());
                    }
                }
                if !matches!(vt, Ty::Int | Ty::Float | Ty::Str | Ty::Bool) {
                    return Err(Diag::new(
                        value.span(),
                        format!("'switch' expects int, float, bool or string, found '{}'", vt),
                    ));
                }
                let tmp = format!("_sw{}", self.switch_seq);
                self.switch_seq += 1;
                let ct = c_ty(&vt);
                self.out.push_str(&format!("    {} {} = {};\n", ct, tmp, vc));
                let mut first = true;
                for case in cases {
                    if case.patterns.is_empty() {
                        return Err(Diag::new(case.span, "'case' needs at least one pattern"));
                    }
                    let mut conds = Vec::new();
                    for p in &case.patterns {
                        let e = as_value_pattern(p);
                        let (pc, pt) = self.expr(&e)?;
                        let pc = self.coerce(&pc, &pt, &vt, e.span())?;
                        let cond = if vt == Ty::Str {
                            format!("(strcmp({}, {}) == 0)", tmp, pc)
                        } else {
                            format!("({} == {})", tmp, pc)
                        };
                        conds.push(cond);
                    }
                    let joined = conds.join(" || ");
                    if first {
                        self.out.push_str(&format!("    if ({}) {{\n", joined));
                        first = false;
                    } else {
                        self.out.push_str(&format!(" else if ({}) {{\n", joined));
                    }
                    self.emit_block_inner(&case.body)?;
                    self.out.push_str("    }");
                }
                if let Some(db) = default {
                    self.out.push_str(" else {\n");
                    self.emit_block_inner(db)?;
                    self.out.push_str("    }");
                }
                self.out.push('\n');
            }
        }
        Ok(())
    }

    /// Lower an enum value switch onto the runtime tag, binding each payload
    /// value in the arm that matched it.
    fn emit_enum_switch(
        &mut self,
        enum_name: &str,
        vc: &str,
        cases: &[SwitchCase],
        default: Option<&Block>,
        span: Span,
    ) -> Result<(), Diag> {
        let info = match self.enums.get(enum_name) {
            Some(i) => i.clone(),
            None => return Err(Diag::new(span, format!("unknown type '{}'", enum_name))),
        };
        let tmp = format!("_sw{}", self.switch_seq);
        self.switch_seq += 1;
        self.out.push_str(&format!("    int64_t {} = ({}).tag;\n", tmp, vc));

        let mut covered: HashSet<usize> = HashSet::new();
        let mut first = true;
        for case in cases {
            if case.patterns.is_empty() {
                return Err(Diag::new(case.span, "'case' needs at least one pattern"));
            }
            if case.patterns.len() > 1
                && case.patterns.iter().any(|p| !pattern_bindings(p).is_empty())
            {
                return Err(Diag::new(
                    case.span,
                    "a 'case' that binds values may list only one pattern",
                ));
            }
            let mut conds = Vec::new();
            let mut decls = String::new();
            let mut bound: Vec<(String, Ty)> = Vec::new();
            for p in &case.patterns {
                let (name, bindings, pspan) = match p {
                    Pattern::Bare { name, span } => (name.clone(), Vec::new(), *span),
                    Pattern::Variant { name, bindings, span } => {
                        (name.clone(), bindings.clone(), *span)
                    }
                    Pattern::Value(_) => {
                        return Err(Diag::new(
                            p.span(),
                            format!("expected a variant of enum '{}', found a value expression", enum_name),
                        ));
                    }
                };
                let Some(vi) = self.variants.get(&name).cloned() else {
                    return Err(Diag::new(pspan, format!("'{}' is not a variant of enum '{}'", name, enum_name)));
                };
                if vi.enum_name != enum_name {
                    return Err(Diag::new(pspan, format!("'{}' is not a variant of enum '{}'", name, enum_name)));
                }
                let want = info.params[vi.index].clone();
                if bindings.is_empty() && !want.is_empty() {
                    return Err(Diag::new(
                        pspan,
                        format!(
                            "variant '{}' carries {} value{}, so '{}' needs {} binding{}",
                            name,
                            want.len(),
                            plural(want.len()),
                            name,
                            want.len(),
                            plural(want.len())
                        ),
                    ));
                }
                if !bindings.is_empty() && bindings.len() != want.len() {
                    return Err(Diag::new(
                        pspan,
                        format!(
                            "variant '{}' carries {} value{}, but {} binding{} given",
                            name,
                            want.len(),
                            plural(want.len()),
                            bindings.len(),
                            plural(bindings.len())
                        ),
                    ));
                }
                for (i, b) in bindings.iter().enumerate() {
                    decls.push_str(&format!(
                        "        {} {} = ({}).u._v{}._{};\n",
                        c_ty(&want[i]),
                        c_var(b),
                        vc,
                        vi.index,
                        i
                    ));
                    bound.push((b.clone(), want[i].clone()));
                }
                covered.insert(vi.index);
                conds.push(format!("({} == {})", tmp, vi.index));
            }
            if first {
                self.out.push_str(&format!("    if ({}) {{\n", conds.join(" || ")));
                first = false;
            } else {
                self.out.push_str(&format!(" else if ({}) {{\n", conds.join(" || ")));
            }
            let mut scope = HashMap::new();
            for (b, ty) in bound {
                scope.insert(
                    b,
                    VarInfo { ty, mutable: false, used: false, span: case.span },
                );
            }
            self.scopes.push(scope);
            self.out.push_str(&decls);
            self.emit_block_inner(&case.body)?;
            self.out.push_str("    }");
            self.scopes.pop();
        }
        if let Some(db) = default {
            self.out.push_str(" else {\n");
            self.emit_block_inner(db)?;
            self.out.push_str("    }");
        }
        self.out.push('\n');

        if default.is_none() && !first {
            let missing: Vec<&str> = (0..info.names.len())
                .filter(|i| !covered.contains(i))
                .map(|i| info.names[i].as_str())
                .collect();
            if !missing.is_empty() {
                return Err(Diag::new(
                    span,
                    format!(
                        "match on '{}' is not exhaustive; {} not covered: {}",
                        enum_name,
                        if missing.len() == 1 { "variant" } else { "variants" },
                        missing.join(", ")
                    ),
                ));
            }
        }
        if default.is_none() && !first {
            self.exhaustive_switches.push(span);
        }
        Ok(())
    }

    /// `Circle(1.0)` -> `((S_Shape){ .tag = 0, .u = { ._v0 = { (double)(1.0) } } })`
    fn variant_ctor(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
        v: &VariantInfo,
    ) -> Result<(String, Ty), Diag> {
        if args.len() != v.params.len() {
            return Err(Diag::new(
                span,
                format!(
                    "'{}' carries {} value{}, but {} {} given",
                    name,
                    v.params.len(),
                    plural(v.params.len()),
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" }
                ),
            ));
        }
        let mut vals = Vec::new();
        for (a, pty) in args.iter().zip(v.params.iter()) {
            let (code, ty) = self.expr_hint(a, Some(pty))?;
            let code = self.coerce(&code, &ty, pty, a.span())?;
            vals.push(code);
        }
        let mut init = format!("((S_{}){{ .tag = {}", v.enum_name, v.index);
        if !vals.is_empty() {
            init.push_str(&format!(", .u = {{ ._v{} = {{ {} }} }}", v.index, vals.join(", ")));
        }
        init.push_str(" })");
        Ok((init, Ty::Named(v.enum_name.clone())))
    }

    fn emit_block_inner(&mut self, b: &Block) -> Result<(), Diag> {
        self.scopes.push(HashMap::new());
        let mut terminated = false;
        let mut warned_unreachable = false;
        for stmt in &b.stmts {
            if terminated && !warned_unreachable {
                self.warnings.push(Diag::warning(
                    stmt.span(),
                    "unreachable code: this statement can never be executed",
                ));
                warned_unreachable = true;
            }
            if stmt_hard_terminates(stmt) {
                terminated = true;
            }
            let depth = self.scopes.len();
            if let Err(e) = self.emit_stmt(stmt) {
                self.errors.push(e);
                while self.scopes.len() > depth {
                    self.scopes.pop();
                }
            }
        }
        self.warn_unused_in_top_scope();
        self.scopes.pop();
        Ok(())
    }

    fn emit_expr_stmt(&mut self, e: &Expr) -> Result<(), Diag> {
        if let Expr::Call { name, args, .. } = e {
            if name == "print" {
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str("    printf(\" \");\n");
                    }
                    let (code, ty) = self.expr(a)?;
                    match ty {
                        Ty::Int => self
                            .out
                            .push_str(&format!("    printf(\"%lld\", (long long)({}));\n", code)),
                        Ty::Float => self
                            .out
                            .push_str(&format!("    printf(\"%.15g\", (double)({}));\n", code)),
                        Ty::Bool => self.out.push_str(&format!(
                            "    printf(\"%s\", ({}) ? \"true\" : \"false\");\n",
                            code
                        )),
                        Ty::Str => self.out.push_str(&format!("    printf(\"%s\", ({}));\n", code)),
                        Ty::Array(_) => {
                            self.out.push_str(&format!("    iks_print_arr({});\n", code))
                        }
                        Ty::Vec2 | Ty::Vec3 | Ty::Vec4 => {
                            let fnm = match ty {
                                Ty::Vec2 => "iks_v2_str",
                                Ty::Vec3 => "iks_v3_str",
                                _ => "iks_v4_str",
                            };
                            self.out
                                .push_str(&format!("    printf(\"%s\", {}({}));\n", fnm, code))
                        }
                        Ty::Void => {
                            return Err(Diag::new(a.span(), "cannot print a value of type 'void'"));
                        }
                        other => {
                            return Err(Diag::new(
                                a.span(),
                                format!("cannot print a value of type '{}'", other),
                            ));
                        }
                    }
                }
                self.out.push_str("    printf(\"\\n\");\n");
                return Ok(());
            }
        }
        let (code, ty) = self.expr(e)?;
        if ty != Ty::Void && !matches!(e, Expr::Call { .. }) {
            return Err(Diag::new(
                e.span(),
                "invalid expression statement: only function calls can stand alone as statements",
            ));
        }
        self.out.push_str(&format!("    {};\n", code));
        Ok(())
    }

    fn emit_assign(&mut self, target: &Expr, value: &Expr, _span: Span) -> Result<(), Diag> {
        match target {
            Expr::Var(name, vspan) => {
                let Some(info) = self.lookup(name) else {
                    return Err(Diag::new(*vspan, format!("undefined variable '{}'", name)));
                };
                if !info.mutable {
                    return Err(Diag::new(
                        *vspan,
                        format!(
                            "cannot assign to '{}': it is declared with 'let' (immutable)",
                            name
                        ),
                    ));
                }
                let ty = info.ty.clone();
                let (code, vty) = self.expr_hint(value, Some(&ty))?;
                let code = self.coerce(&code, &vty, &ty, value.span())?;
                self.out.push_str(&format!("    {} = {};\n", c_var(name), code));
                Ok(())
            }
            Expr::Field(base, fname, fspan) => {
                let (bcode, bty) = self.expr(base)?;
                if bty.is_vec() {
                    let dim = bty.vec_dim().unwrap();
                    let idx = match fname.as_str() {
                        "x" => Some(0usize),
                        "y" => Some(1),
                        "z" => Some(2),
                        "w" => Some(3),
                        _ => None,
                    };
                    if !matches!(idx, Some(i) if i < dim) {
                        return Err(Diag::new(
                            *fspan,
                            format!("type '{}' has no component '{}'", bty, fname),
                        ));
                    }
                    let (vcode, vty) = self.expr_hint(value, Some(&Ty::Float))?;
                    let vcode = self.coerce(&vcode, &vty, &Ty::Float, value.span())?;
                    self.out
                        .push_str(&format!("    ({}).{} = {};\n", bcode, fname, vcode));
                    return Ok(());
                }
                let Ty::Named(sname) = &bty else {
                    return Err(Diag::new(
                        base.span(),
                        format!("type '{}' has no fields", bty),
                    ));
                };
                let Some(info) = self.structs.get(sname) else {
                    return Err(Diag::new(*fspan, format!("unknown type '{}'", sname)));
                };
                let Some((_, fty)) = info.fields.iter().find(|(n, _)| n == fname) else {
                    return Err(Diag::new(
                        *fspan,
                        format!("struct '{}' has no field '{}'", sname, fname),
                    ));
                };
                let fty = fty.clone();
                let (vcode, vty) = self.expr_hint(value, Some(&fty))?;
                let vcode = self.coerce(&vcode, &vty, &fty, value.span())?;
                self.out
                    .push_str(&format!("    {}.{} = {};\n", bcode, mangle_field(fname), vcode));
                Ok(())
            }
            Expr::Index(base, index, _) => {
                let (bcode, bty) = self.expr(base)?;
                let elem = match &bty {
                    Ty::Array(e) => (**e).clone(),
                    other => {
                        return Err(Diag::new(
                            base.span(),
                            format!("cannot index into a value of type '{}'", other),
                        ));
                    }
                };
                let (icode, ity) = self.expr(index)?;
                self.coerce(&icode, &ity, &Ty::Int, index.span())?;
                let (vcode, vty) = self.expr_hint(value, Some(&elem))?;
                let vcode = self.coerce(&vcode, &vty, &elem, value.span())?;
                self.out.push_str(&format!(
                    "    *({}*)iks_idx({}, {}) = {};\n",
                    c_ty(&elem),
                    bcode,
                    icode,
                    vcode
                ));
                Ok(())
            }
            other => Err(Diag::new(other.span(), "invalid assignment target")),
        }
    }

    fn lookup(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn cond_code(&self, code: &str, ty: &Ty, span: Span) -> Result<String, Diag> {
        match ty {
            Ty::Bool => Ok(code.to_string()),
            Ty::Int => Ok(format!("(({}) != 0)", code)),
            other => Err(Diag::new(
                span,
                format!("condition must be 'bool' or 'int', found '{}'", other),
            )),
        }
    }

    fn coerce(&self, code: &str, from: &Ty, to: &Ty, span: Span) -> Result<String, Diag> {
        if from == to {
            return Ok(code.to_string());
        }
        match (from, to) {
            (Ty::Int, Ty::Float) => Ok(format!("((double)({}))", code)),
            (Ty::Void, _) => Err(Diag::new(span, "cannot use a value of type 'void'")),
            _ => Err(Diag::new(
                span,
                format!("type mismatch: expected '{}', found '{}'", to, from),
            )),
        }
    }

    fn expr(&mut self, e: &Expr) -> Result<(String, Ty), Diag> {
        self.expr_hint(e, None)
    }

    fn expr_hint(&mut self, e: &Expr, hint: Option<&Ty>) -> Result<(String, Ty), Diag> {
        match e {
            Expr::Int(v, _) => Ok((format!("{}", v), Ty::Int)),
            Expr::Float(v, _) => Ok((format!("{:?}", v), Ty::Float)),
            Expr::Str(s, _) => Ok((format!("\"{}\"", escape_c(s)), Ty::Str)),
            Expr::Bool(b, _) => Ok((if *b { "true" } else { "false" }.to_string(), Ty::Bool)),
            Expr::Nil(_) => Ok(("((void*)0)".to_string(), Ty::Ptr)),
            Expr::Var(name, span) => {
                let mut found: Option<Ty> = None;
                for scope in self.scopes.iter_mut().rev() {
                    if let Some(v) = scope.get_mut(name) {
                        v.used = true;
                        found = Some(v.ty.clone());
                        break;
                    }
                }
                match found {
                    Some(ty) => Ok((c_var(name), ty)),
                    None => {
                        if let Some(v) = self.variants.get(name).cloned() {
                            if v.params.is_empty() {
                                return Ok((
                                    format!("((S_{}){{ .tag = {} }})", v.enum_name, v.index),
                                    Ty::Named(v.enum_name.clone()),
                                ));
                            }
                            return Err(Diag::new(
                                *span,
                                format!(
                                    "variant '{}' carries {} value{}; write '{}(...)'",
                                    name,
                                    v.params.len(),
                                    plural(v.params.len()),
                                    name
                                ),
                            ));
                        }
                        Err(Diag::new(*span, format!("undefined variable '{}'", name)))
                    }
                }
            }
            Expr::Array(items, span) => self.array_lit(items, *span, hint),
            Expr::Tuple(items, span) => {
                let mut tys = Vec::new();
                let mut codes = Vec::new();
                for item in items {
                    let (c, t) = self.expr(item)?;
                    tys.push(t);
                    codes.push(c);
                }
                let ty = Ty::Tuple(tys);
                let cstruct = c_ty(&ty);
                self.ensure_tuple_typedef(&ty, *span)?;
                Ok((format!("(({}){{ {} }})", cstruct, codes.join(", ")), ty))
            }
            Expr::Field(base, fname, fspan) => {
                let (bcode, bty) = self.expr(base)?;
                if let Ty::Tuple(ref ts) = bty {
                    let Ok(idx) = fname.parse::<usize>() else {
                        return Err(Diag::new(*fspan, format!("tuple field must be a number, found '{}'", fname)));
                    };
                    if idx >= ts.len() {
                        return Err(Diag::new(*fspan, format!("tuple index {} out of range (tuple has {} elements)", idx, ts.len())));
                    }
                    let _ = c_ty(&bty);
                    self.ensure_tuple_typedef(&bty, *fspan)?;
                    return Ok((format!("{}.{}", bcode, format!("_{}", idx)), ts[idx].clone()));
                }
                if bty.is_vec() {
                    let dim = bty.vec_dim().unwrap();
                    let idx = match fname.as_str() {
                        "x" => Some(0usize),
                        "y" => Some(1),
                        "z" => Some(2),
                        "w" => Some(3),
                        _ => None,
                    };
                    return match idx {
                        Some(i) if i < dim => Ok((format!("({}).{}", bcode, fname), Ty::Float)),
                        _ => Err(Diag::new(
                            *fspan,
                            format!("type '{}' has no component '{}'", bty, fname),
                        )),
                    };
                }
                let Ty::Named(sname) = &bty else {
                    return Err(Diag::new(base.span(), format!("type '{}' has no fields", bty)));
                };
                let Some(info) = self.structs.get(sname) else {
                    return Err(Diag::new(*fspan, format!("unknown type '{}'", sname)));
                };
                let Some((_, fty)) = info.fields.iter().find(|(n, _)| n == fname) else {
                    return Err(Diag::new(
                        *fspan,
                        format!("struct '{}' has no field '{}'", sname, fname),
                    ));
                };
                Ok((format!("{}.{}", bcode, mangle_field(fname)), fty.clone()))
            }
            Expr::StructLit { name, fields, span } => self.struct_lit(name, fields, *span),
            Expr::Index(base, index, _) => {
                let (bcode, bty) = self.expr(base)?;
                let elem = match &bty {
                    Ty::Array(e) => (**e).clone(),
                    other => {
                        return Err(Diag::new(
                            base.span(),
                            format!("cannot index into a value of type '{}'", other),
                        ));
                    }
                };
                let (icode, ity) = self.expr(index)?;
                let icode = self.coerce(&icode, &ity, &Ty::Int, index.span())?;
                Ok((
                    format!("*({}*)iks_idx({}, {})", c_ty(&elem), bcode, icode),
                    elem,
                ))
            }
            Expr::Unary { op, operand, span } => {
                let (code, ty) = self.expr(operand)?;
                match op {
                    UnOp::Neg => {
                        if !ty.is_numeric() {
                            return Err(Diag::new(
                                *span,
                                format!("unary '-' expects a number, found '{}'", ty),
                            ));
                        }
                        Ok((format!("(-({}))", code), ty))
                    }
                    UnOp::Not => match ty {
                        Ty::Bool => Ok((format!("(!({}))", code), Ty::Bool)),
                        Ty::Int => Ok((format!("(({}) == 0)", code), Ty::Bool)),
                        other => Err(Diag::new(
                            *span,
                            format!("unary '!' expects 'bool' or 'int', found '{}'", other),
                        )),
                    },
                    UnOp::BitNot => {
                        if ty != Ty::Int {
                            return Err(Diag::new(
                                *span,
                                format!("unary '~' expects 'int', found '{}'", ty),
                            ));
                        }
                        Ok((format!("((~({})))", code), Ty::Int))
                    }
                }
            }
            Expr::Binary { op, lhs, rhs, span } => self.binary(*op, lhs, rhs, *span),
            Expr::Call { name, args, span } => self.call(name, args, *span),
        }
    }

    fn struct_lit(
        &mut self,
        name: &str,
        fields: &[(String, Expr, Span)],
        span: Span,
    ) -> Result<(String, Ty), Diag> {
        let Some(info) = self.structs.get(name).cloned() else {
            return Err(Diag::new(span, format!("unknown type '{}'", name)));
        };
        let mut seen: Vec<String> = Vec::new();
        for (fname, _, fspan) in fields {
            if seen.contains(fname) {
                return Err(Diag::new(*fspan, format!("field '{}' is initialized twice", fname)));
            }
            if !info.fields.iter().any(|(n, _)| n == fname) {
                return Err(Diag::new(
                    *fspan,
                    format!("struct '{}' has no field '{}'", name, fname),
                ));
            }
            seen.push(fname.clone());
        }
        let mut parts = Vec::new();
        for (fname, fty) in &info.fields {
            let Some((_, expr, _)) = fields.iter().find(|(n, _, _)| n == fname) else {
                return Err(Diag::new(
                    span,
                    format!("missing field '{}' in '{}' literal", fname, name),
                ));
            };
            let (code, ty) = self.expr_hint(expr, Some(fty))?;
            let code = self.coerce(&code, &ty, fty, expr.span())?;
            let _ = fty;
            parts.push(format!(".{} = {}", mangle_field(fname), code));
        }
        Ok((format!("((S_{}){{ {} }})", name, parts.join(", ")), Ty::Named(name.to_string())))
    }

    fn binary(
        &mut self,
        op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
        span: Span,
    ) -> Result<(String, Ty), Diag> {
        let (lc, lt) = self.expr(lhs)?;
        let (rc, rt) = self.expr(rhs)?;
        use BinOp::*;
        match op {
            Add => {
                if lt.is_vec() || rt.is_vec() {
                    return self.vec_binary(&lc, &lt, &rc, &rt, span, '+');
                }
                if lt == Ty::Str && rt == Ty::Str {
                    return Ok((format!("iks_cat({}, {})", lc, rc), Ty::Str));
                }
                if lt == Ty::Str || rt == Ty::Str {
                    return Err(Diag::new(
                        span,
                        format!(
                            "cannot add '{}' and '{}'; use str(x) to convert a value to string",
                            lt, rt
                        ),
                    ));
                }
                self.numeric_binary(&lc, &lt, &rc, &rt, span, "+")
            }
            Sub | Mul | Div => {
                if lt.is_vec() || rt.is_vec() {
                    let opc = match op {
                        Sub => '-',
                        Mul => '*',
                        _ => '/',
                    };
                    return self.vec_binary(&lc, &lt, &rc, &rt, span, opc);
                }
                if !lt.is_numeric() || !rt.is_numeric() {
                    return Err(Diag::new(
                        span,
                        format!("operator expects numbers, found '{}' and '{}'", lt, rt),
                    ));
                }
                if op == Div && lt == Ty::Int && rt == Ty::Int {
                    return Ok((format!("iks_div_i({}, {})", lc, rc), Ty::Int));
                }
                self.numeric_binary(
                    &lc,
                    &lt,
                    &rc,
                    &rt,
                    span,
                    match op {
                        Sub => "-",
                        Mul => "*",
                        _ => "/",
                    },
                )
            }
            Mod => {
                if lt != Ty::Int || rt != Ty::Int {
                    return Err(Diag::new(
                        span,
                        format!("'%' expects int operands, found '{}' and '{}'", lt, rt),
                    ));
                }
                Ok((format!("iks_mod_i({}, {})", lc, rc), Ty::Int))
            }
            Eq | Ne => {
                let cmp = if op == Eq { "==" } else { "!=" };
                if lt.is_numeric() && rt.is_numeric() {
                    let (lc2, rc2, _) = self.promote(&lc, &lt, &rc, &rt)?;
                    return Ok((format!("(({}) {} ({}))", lc2, cmp, rc2), Ty::Bool));
                }
                if lt == Ty::Bool && rt == Ty::Bool {
                    return Ok((format!("(({}) {} ({}))", lc, cmp, rc), Ty::Bool));
                }
                if lt == Ty::Str && rt == Ty::Str {
                    let opstr = if op == Eq { "==" } else { "!=" };
                    return Ok((format!("(strcmp({}, {}) {} 0)", lc, rc, opstr), Ty::Bool));
                }
                if lt == Ty::Ptr && rt == Ty::Ptr {
                    return Ok((format!("(({}) {} ({}))", lc, cmp, rc), Ty::Bool));
                }
                if lt.is_vec() && rt == lt {
                    let n = lt.vec_dim().unwrap();
                    let eq = format!("iks_v{}_eq({}, {})", n, lc, rc);
                    return Ok((
                        if op == Eq { eq } else { format!("!({})", eq) },
                        Ty::Bool,
                    ));
                }
                Err(Diag::new(span, format!("cannot compare '{}' with '{}'", lt, rt)))
            }
            Lt | Le | Gt | Ge => {
                let cmp = match op {
                    Lt => "<",
                    Le => "<=",
                    Gt => ">",
                    Ge => ">=",
                    _ => unreachable!(),
                };
                if lt.is_numeric() && rt.is_numeric() {
                    let (lc2, rc2, _) = self.promote(&lc, &lt, &rc, &rt)?;
                    return Ok((format!("(({}) {} ({}))", lc2, cmp, rc2), Ty::Bool));
                }
                if lt == Ty::Str && rt == Ty::Str {
                    return Ok((format!("(strcmp({}, {}) {} 0)", lc, rc, cmp), Ty::Bool));
                }
                Err(Diag::new(span, format!("cannot order '{}' against '{}'", lt, rt)))
            }
            And | Or => {
                let lc = self.truthy(&lc, &lt, lhs.span())?;
                let rc = self.truthy(&rc, &rt, rhs.span())?;
                let opstr = if op == And { "&&" } else { "||" };
                Ok((format!("({} {} {})", lc, opstr, rc), Ty::Bool))
            }
            BitAnd | BitOr | BitXor | Shl | Shr => {
                if lt != Ty::Int || rt != Ty::Int {
                    return Err(Diag::new(
                        span,
                        format!(
                            "bitwise operator expects int operands, found '{}' and '{}'",
                            lt, rt
                        ),
                    ));
                }
                let opstr = match op {
                    BitAnd => "&",
                    BitOr => "|",
                    BitXor => "^",
                    Shl => "<<",
                    Shr => ">>",
                    _ => unreachable!(),
                };
                Ok((format!("(({}) {} ({}))", lc, opstr, rc), Ty::Int))
            }
        }
    }

    fn truthy(&self, code: &str, ty: &Ty, span: Span) -> Result<String, Diag> {
        match ty {
            Ty::Bool => Ok(code.to_string()),
            Ty::Int => Ok(format!("(({}) != 0)", code)),
            other => Err(Diag::new(
                span,
                format!("expected 'bool' or 'int', found '{}'", other),
            )),
        }
    }

    fn promote(&self, lc: &str, lt: &Ty, rc: &str, rt: &Ty) -> Result<(String, String, Ty), Diag> {
        if *lt == Ty::Int && *rt == Ty::Int {
            Ok((lc.to_string(), rc.to_string(), Ty::Int))
        } else {
            Ok((
                format!("((double)({}))", lc),
                format!("((double)({}))", rc),
                Ty::Float,
            ))
        }
    }

    fn numeric_binary(
        &self,
        lc: &str,
        lt: &Ty,
        rc: &str,
        rt: &Ty,
        span: Span,
        op: &str,
    ) -> Result<(String, Ty), Diag> {
        if !lt.is_numeric() || !rt.is_numeric() {
            return Err(Diag::new(
                span,
                format!("operator expects numbers, found '{}' and '{}'", lt, rt),
            ));
        }
        let (lc2, rc2, ty) = self.promote(lc, lt, rc, rt)?;
        Ok((format!("(({}) {} ({}))", lc2, op, rc2), ty))
    }

    fn vec_binary(
        &self,
        lc: &str,
        lt: &Ty,
        rc: &str,
        rt: &Ty,
        span: Span,
        op: char,
    ) -> Result<(String, Ty), Diag> {
        let vfn = |dim: usize, suffix: &str| format!("iks_v{}_{}", dim, suffix);
        // vec <op> vec -> component-wise (GLSL-compatible: '*' scales per component,
        // cross products are spelled `cross(a, b)` explicitly).
        if lt.is_vec() && rt == lt {
            let suffix = match op {
                '+' => "add",
                '-' => "sub",
                '*' => "mul",
                '/' => "div",
                _ => "",
            };
            if !suffix.is_empty() {
                let n = lt.vec_dim().unwrap();
                return Ok((format!("{}({}, {})", vfn(n, suffix), lc, rc), lt.clone()));
            }
        }
        // vec <op> scalar
        if lt.is_vec() && rt.is_numeric() {
            let suffix = match op {
                '+' => "adds",
                '-' => "subs",
                '*' => "scale",
                '/' => "divs",
                _ => "",
            };
            if !suffix.is_empty() {
                let n = lt.vec_dim().unwrap();
                let rc2 = self.coerce(rc, rt, &Ty::Float, span)?;
                return Ok((format!("{}({}, {})", vfn(n, suffix), lc, rc2), lt.clone()));
            }
        }
        // scalar <op> vec (only where the operation is well-defined)
        if lt.is_numeric() && rt.is_vec() {
            let suffix = match op {
                '*' => "scale",
                '+' => "adds",
                _ => "",
            };
            if !suffix.is_empty() {
                let n = rt.vec_dim().unwrap();
                let lc2 = self.coerce(lc, lt, &Ty::Float, span)?;
                return Ok((format!("{}({}, {})", vfn(n, suffix), rc, lc2), rt.clone()));
            }
        }
        Err(Diag::new(
            span,
            format!("operator '{}' cannot be applied to '{}' and '{}'", op, lt, rt),
        ))
    }

    fn array_lit(
        &mut self,
        items: &[Expr],
        span: Span,
        hint: Option<&Ty>,
    ) -> Result<(String, Ty), Diag> {
        let ann_elem = match hint {
            Some(Ty::Array(e)) => Some((**e).clone()),
            _ => None,
        };
        let mut codes = Vec::new();
        let mut tys = Vec::new();
        for item in items {
            let (c, t) = self.expr_hint(item, ann_elem.as_ref())?;
            codes.push(c);
            tys.push(t);
        }
        let elem = if let Some(a) = ann_elem {
            for (t, item) in tys.iter().zip(items) {
                self.coerce("(x)", t, &a, item.span())?;
            }
            a
        } else if items.is_empty() {
            return Err(Diag::new(
                span,
                "cannot infer the element type of an empty array; add an annotation, e.g. 'var a: int[] = []'",
            ));
        } else {
            unify_elems(&tys, span)?
        };
        let codes: Vec<String> = codes
            .into_iter()
            .zip(tys.iter())
            .map(|(c, t)| {
                if *t == Ty::Int && elem == Ty::Float {
                    format!("((double)({}))", c)
                } else {
                    c
                }
            })
            .collect();
        let arr_ty = Ty::Array(Box::new(elem.clone()));
        if codes.is_empty() {
            return Ok((
                format!(
                    "iks_arr((int64_t)sizeof({}), (int64_t)0, NULL, {})",
                    c_ty(&elem),
                    tag_of(&elem)
                ),
                arr_ty,
            ));
        }
        let mut lit = String::new();
        lit.push_str("iks_arr((int64_t)sizeof(");
        lit.push_str(&c_ty(&elem));
        lit.push_str("), (int64_t)");
        lit.push_str(&codes.len().to_string());
        lit.push_str(", (void*)((");
        lit.push_str(&c_ty(&elem));
        lit.push_str("[]){");
        lit.push_str(&codes.join(", "));
        lit.push_str("}), ");
        lit.push_str(&tag_of(&elem).to_string());
        lit.push_str(")");
        Ok((lit, arr_ty))
    }

    fn int_args(&mut self, args: &[Expr], want: usize, name: &str, span: Span) -> Result<Vec<String>, Diag> {
        self.check_arity(name, args, want, want, span)?;
        let mut out = Vec::new();
        for a in args {
            let (c, t) = self.expr(a)?;
            out.push(self.coerce(&c, &t, &Ty::Int, a.span())?);
        }
        Ok(out)
    }

    fn float_args(&mut self, args: &[Expr], want: usize, name: &str, span: Span) -> Result<Vec<String>, Diag> {
        self.check_arity(name, args, want, want, span)?;
        let mut out = Vec::new();
        for a in args {
            let (c, t) = self.expr(a)?;
            out.push(self.coerce(&c, &t, &Ty::Float, a.span())?);
        }
        Ok(out)
    }

    fn str_args(&mut self, args: &[Expr], want: usize, name: &str, span: Span) -> Result<Vec<String>, Diag> {
        self.check_arity(name, args, want, want, span)?;
        let mut out = Vec::new();
        for a in args {
            let (c, t) = self.expr(a)?;
            out.push(self.coerce(&c, &t, &Ty::Str, a.span())?);
        }
        Ok(out)
    }

    fn call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<(String, Ty), Diag> {
        match name {
            "print" => Err(Diag::new(
                span,
                "'print' returns no value; call it as a statement, e.g. print(x);",
            )),
            "assert" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (cc, ct) = self.expr(&args[0])?;
                let cc = self.truthy(&cc, &ct, args[0].span())?;
                let (mc, mt) = self.expr(&args[1])?;
                let mc = self.coerce(&mc, &mt, &Ty::Str, args[1].span())?;
                Ok((format!("iks_assert(({}), ({}))", cc, mc), Ty::Void))
            }
            "beep" => {
                let a = self.int_args(args, 2, name, span)?;
                Ok((format!("iks_beep({}, {})", a[0], a[1]), Ty::Void))
            }
            "len" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                match ty {
                    Ty::Array(_) => Ok((format!("({}).len", code), Ty::Int)),
                    Ty::Str => Ok((format!("((int64_t)strlen({}))", code), Ty::Int)),
                    other => Err(Diag::new(
                        args[0].span(),
                        format!("'len' expects an array or a string, found '{}'", other),
                    )),
                }
            }
            "str" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                match ty {
                    Ty::Int => Ok((format!("iks_str_i({})", code), Ty::Str)),
                    Ty::Float => Ok((format!("iks_str_f({})", code), Ty::Str)),
                    Ty::Bool => Ok((format!("iks_str_b({})", code), Ty::Str)),
                    Ty::Str => Ok((code, Ty::Str)),
                    Ty::Vec2 => Ok((format!("iks_v2_str({})", code), Ty::Str)),
                    Ty::Vec3 => Ok((format!("iks_v3_str({})", code), Ty::Str)),
                    Ty::Vec4 => Ok((format!("iks_v4_str({})", code), Ty::Str)),
                    other => Err(Diag::new(
                        args[0].span(),
                        format!("'str' expects int, float, bool or string, found '{}'", other),
                    )),
                }
            }
            "int" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                match ty {
                    Ty::Int => Ok((code, Ty::Int)),
                    Ty::Float => Ok((format!("((int64_t)({}))", code), Ty::Int)),
                    Ty::Bool => Ok((format!("((int64_t)({}))", code), Ty::Int)),
                    Ty::Str => Ok((format!("iks_parse_i({})", code), Ty::Int)),
                    other => Err(Diag::new(
                        args[0].span(),
                        format!("'int' expects int, float, bool or string, found '{}'", other),
                    )),
                }
            }
            "float" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                match ty {
                    Ty::Float => Ok((code, Ty::Float)),
                    Ty::Int => Ok((format!("((double)({}))", code), Ty::Float)),
                    Ty::Str => Ok((format!("iks_parse_f({})", code), Ty::Float)),
                    other => Err(Diag::new(
                        args[0].span(),
                        format!("'float' expects int, float or string, found '{}'", other),
                    )),
                }
            }
            "input" => {
                self.check_arity(name, args, 0, 1, span)?;
                if args.is_empty() {
                    return Ok(("iks_input(\"\")".to_string(), Ty::Str));
                }
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Str, args[0].span())?;
                Ok((format!("iks_input({})", code), Ty::Str))
            }
            "push" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let elem = match &ty {
                    Ty::Array(e) => (**e).clone(),
                    other => {
                        return Err(Diag::new(
                            args[0].span(),
                            format!("'push' expects an array, found '{}'", other),
                        ));
                    }
                };
                let (vcode, vty) = self.expr_hint(&args[1], Some(&elem))?;
                let vcode = self.coerce(&vcode, &vty, &elem, args[1].span())?;
                Ok((
                    format!("iks_push({}, &({}){{{}}})", code, c_ty(&elem), vcode),
                    ty,
                ))
            }
            "rand" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_rand()".to_string(), Ty::Float))
            }
            "exit" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Int, args[0].span())?;
                Ok((format!("exit((int)({}))", code), Ty::Void))
            }
            "sleep" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Int, args[0].span())?;
                Ok((format!("iks_sleep({})", code), Ty::Void))
            }
            "sin" | "cos" | "sqrt" | "abs" | "floor" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Float, args[0].span())?;
                let cfn = match name { "abs" => "fabs", other => other };
                Ok((format!("{}({})", cfn, code), Ty::Float))
            }
            "pow" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (c1, t1) = self.expr(&args[0])?;
                let c1 = self.coerce(&c1, &t1, &Ty::Float, args[0].span())?;
                let (c2, t2) = self.expr(&args[1])?;
                let c2 = self.coerce(&c2, &t2, &Ty::Float, args[1].span())?;
                Ok((format!("pow({}, {})", c1, c2), Ty::Float))
            }
            "gfx_open" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (wc, wty) = self.expr(&args[0])?;
                let wc = self.coerce(&wc, &wty, &Ty::Int, args[0].span())?;
                let (hc, hty) = self.expr(&args[1])?;
                let hc = self.coerce(&hc, &hty, &Ty::Int, args[1].span())?;
                let (tcode, tty) = self.expr(&args[2])?;
                let tcode = self.coerce(&tcode, &tty, &Ty::Str, args[2].span())?;
                Ok((format!("iks_gfx_open({}, {}, {})", wc, hc, tcode), Ty::Void))
            }
            "gfx_poll" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_gfx_poll()".to_string(), Ty::Bool))
            }
            "gfx_present" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_gfx_present()".to_string(), Ty::Void))
            }
            "gfx_clear" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_gfx_clear({})", a.join(", ")), Ty::Void))
            }
            "gfx_add" | "gfx_pixel" => {
                let a = self.int_args(args, 5, name, span)?;
                let cfn = if name == "gfx_add" { "iks_gfx_add" } else { "iks_gfx_pixel" };
                Ok((format!("{}({})", cfn, a.join(", ")), Ty::Void))
            }
            "gfx_line" => {
                let a = self.int_args(args, 7, name, span)?;
                Ok((format!("iks_gfx_line({})", a.join(", ")), Ty::Void))
            }
            "gfx_width" | "gfx_height" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = if name == "gfx_width" { "iks_gfx_width" } else { "iks_gfx_height" };
                Ok((format!("{}()", cfn), Ty::Int))
            }
            "gfx_time" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_gfx_time()".to_string(), Ty::Float))
            }
            "gfx_mouse_x" | "gfx_mouse_y" | "touch_x" | "touch_y" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = match name {
                    "gfx_mouse_x" | "touch_x" => "iks_gfx_mouse_x",
                    _ => "iks_gfx_mouse_y",
                };
                Ok((format!("{}()", cfn), Ty::Int))
            }
            "gfx_key_down" => {
                let a = self.int_args(args, 1, name, span)?;
                Ok((format!("iks_gfx_key_down({})", a[0]), Ty::Bool))
            }
            "gl_init" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (wc, wty) = self.expr(&args[0])?;
                let wc = self.coerce(&wc, &wty, &Ty::Int, args[0].span())?;
                let (hc, hty) = self.expr(&args[1])?;
                let hc = self.coerce(&hc, &hty, &Ty::Int, args[1].span())?;
                let (tcode, tty) = self.expr(&args[2])?;
                let tcode = self.coerce(&tcode, &tty, &Ty::Str, args[2].span())?;
                Ok((format!("iks_gl_init({}, {}, {})", wc, hc, tcode), Ty::Void))
            }
            "gl_clear" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_gl_clear({})", a.join(", ")), Ty::Void))
            }
            "gl_view" => {
                let a = self.int_args(args, 4, name, span)?;
                Ok((format!("iks_gl_view({})", a.join(", ")), Ty::Void))
            }
            "gl_proj" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_gl_proj({})", a.join(", ")), Ty::Void))
            }
            "gl_push" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_gl_push()".to_string(), Ty::Void))
            }
            "gl_pop" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_gl_pop()".to_string(), Ty::Void))
            }
            "gl_translate" => {
                let a = self.float_args(args, 3, name, span)?;
                Ok((format!("iks_gl_translate({})", a.join(", ")), Ty::Void))
            }
            "gl_rotate" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_gl_rotate({})", a.join(", ")), Ty::Void))
            }
            "gl_scale" => {
                let a = self.float_args(args, 3, name, span)?;
                Ok((format!("iks_gl_scale({})", a.join(", ")), Ty::Void))
            }
            "gl_color" => {
                let a = self.int_args(args, 4, name, span)?;
                Ok((format!("iks_gl_color({})", a.join(", ")), Ty::Void))
            }
            "gl_begin" => {
                let a = self.int_args(args, 1, name, span)?;
                Ok((format!("iks_gl_begin({})", a[0]), Ty::Void))
            }
            "gl_vertex" => {
                let a = self.float_args(args, 3, name, span)?;
                Ok((format!("iks_gl_vertex({})", a.join(", ")), Ty::Void))
            }
            "gl_end" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_gl_end()".to_string(), Ty::Void))
            }
            "gl_point_size" | "gl_line_width" => {
                let a = self.float_args(args, 1, name, span)?;
                let cfn = if name == "gl_point_size" { "iks_gl_point_size" } else { "iks_gl_line_width" };
                Ok((format!("{}({})", cfn, a[0]), Ty::Void))
            }
            "gl_depth" | "gl_blend" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                let c = self.truthy(&c, &t, args[0].span())?;
                let cfn = if name == "gl_depth" { "iks_gl_depth" } else { "iks_gl_blend" };
                Ok((format!("{}(({}) ? 1 : 0)", cfn, c), Ty::Void))
            }
            "gl_swap" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_gl_swap()".to_string(), Ty::Int))
            }
            "gl_title" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                let c = self.coerce(&c, &t, &Ty::Str, args[0].span())?;
                Ok((format!("iks_gl_title({})", c), Ty::Void))
            }
            "gl_fullscreen" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                let c = self.truthy(&c, &t, args[0].span())?;
                Ok((format!("iks_gl_fullscreen(({}) ? 1 : 0)", c), Ty::Void))
            }
            "gl_width" | "gl_height" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = if name == "gl_width" { "iks_gl_width" } else { "iks_gl_height" };
                Ok((format!("{}()", cfn), Ty::Int))
            }
            "gl_proj_ortho" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_gl_proj_ortho({})", a.join(", ")), Ty::Void))
            }
            "sprite_load" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (p, pty) = self.expr(&args[0])?;
                let p = self.coerce(&p, &pty, &Ty::Str, args[0].span())?;
                self.sprite_used = true;
                Ok((format!("iks_sprite_load({})", p), Ty::Int))
            }
            "sprite_draw" => {
                self.check_arity(name, args, 5, 5, span)?;
                let (ic, ity) = self.expr(&args[0])?;
                let ic = self.coerce(&ic, &ity, &Ty::Int, args[0].span())?;
                let mut coords: Vec<String> = Vec::new();
                for i in 1..5 {
                    let (c, t) = self.expr(&args[i])?;
                    let c = self.coerce(&c, &t, &Ty::Float, args[i].span())?;
                    coords.push(c);
                }
                self.sprite_used = true;
                Ok((format!("iks_sprite_draw({}, {})", ic, coords.join(", ")), Ty::Void))
            }
            "sprite_width" | "sprite_height" => {
                let a = self.int_args(args, 1, name, span)?;
                self.sprite_used = true;
                let cfn = if name == "sprite_width" { "iks_sprite_width" } else { "iks_sprite_height" };
                Ok((format!("{}({})", cfn, a[0]), Ty::Int))
            }
            "sprite_unload" => {
                let a = self.int_args(args, 1, name, span)?;
                self.sprite_used = true;
                Ok((format!("iks_sprite_unload({})", a[0]), Ty::Void))
            }
            "cpu_load" | "gpu_load" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = if name == "cpu_load" { "iks_cpu_load" } else { "iks_gpu_load" };
                Ok((format!("{}()", cfn), Ty::Float))
            }
            "npu_available" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_npu_available()".to_string(), Ty::Bool))
            }
            "npu_count" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_npu_count()".to_string(), Ty::Int))
            }
            "npu_name" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_npu_name()".to_string(), Ty::Str))
            }
            "npu_tops" | "npu_load" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = if name == "npu_tops" { "iks_npu_tops" } else { "iks_npu_load" };
                Ok((format!("{}()", cfn), Ty::Float))
            }
            "d3d_init" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (wc, wty) = self.expr(&args[0])?;
                let wc = self.coerce(&wc, &wty, &Ty::Int, args[0].span())?;
                let (hc, hty) = self.expr(&args[1])?;
                let hc = self.coerce(&hc, &hty, &Ty::Int, args[1].span())?;
                let (tcode, tty) = self.expr(&args[2])?;
                let tcode = self.coerce(&tcode, &tty, &Ty::Str, args[2].span())?;
                Ok((format!("iks_d3d_init({}, {}, {})", wc, hc, tcode), Ty::Void))
            }
            "d3d_clear" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_d3d_clear({})", a.join(", ")), Ty::Void))
            }
            "d3d_draw" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                match &t {
                    Ty::Array(e) if **e == Ty::Float => Ok((format!("iks_d3d_draw({})", c), Ty::Void)),
                    other => Err(Diag::new(
                        args[0].span(),
                        format!("'d3d_draw' expects a float[], found '{}'", other),
                    )),
                }
            }
            "d3d_present" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_d3d_present()".to_string(), Ty::Int))
            }
            "d3d_color" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_d3d_color({})", a.join(", ")), Ty::Void))
            }
            "d3d_begin" => {
                let a = self.int_args(args, 1, name, span)?;
                Ok((format!("iks_d3d_begin({})", a[0]), Ty::Void))
            }
            "d3d_vertex" => {
                let a = self.float_args(args, 3, name, span)?;
                Ok((format!("iks_d3d_vertex({})", a.join(", ")), Ty::Void))
            }
            "d3d_end" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_d3d_end()".to_string(), Ty::Void))
            }
            "d3d_title" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                let c = self.coerce(&c, &t, &Ty::Str, args[0].span())?;
                Ok((format!("iks_d3d_title({})", c), Ty::Void))
            }
            "d3d_width" | "d3d_height" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = if name == "d3d_width" { "iks_d3d_width" } else { "iks_d3d_height" };
                Ok((format!("{}()", cfn), Ty::Int))
            }
            "d3d_rect" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_d3d_rect({})", a.join(", ")), Ty::Void))
            }
            "d3d_circle" => {
                let a = self.float_args(&args[0..3], 3, name, span)?;
                let (sc, sty) = self.expr(&args[3])?;
                let sc = self.coerce(&sc, &sty, &Ty::Int, args[3].span())?;
                Ok((format!("iks_d3d_circle({}, {}, {}, {})", a[0], a[1], a[2], sc), Ty::Void))
            }
            "d3d_line" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_d3d_line({})", a.join(", ")), Ty::Void))
            }
            "dx12_init" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (wc, wty) = self.expr(&args[0])?;
                let wc = self.coerce(&wc, &wty, &Ty::Int, args[0].span())?;
                let (hc, hty) = self.expr(&args[1])?;
                let hc = self.coerce(&hc, &hty, &Ty::Int, args[1].span())?;
                let (tcode, tty) = self.expr(&args[2])?;
                let tcode = self.coerce(&tcode, &tty, &Ty::Str, args[2].span())?;
                Ok((format!("iks_d12_init({}, {}, {})", wc, hc, tcode), Ty::Void))
            }
            "dx12_clear" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_d12_clear({})", a.join(", ")), Ty::Void))
            }
            "dx12_color" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_d12_color({})", a.join(", ")), Ty::Void))
            }
            "dx12_begin" => {
                let a = self.int_args(args, 1, name, span)?;
                Ok((format!("iks_d12_begin({})", a[0]), Ty::Void))
            }
            "dx12_vertex" => {
                let a = self.float_args(args, 3, name, span)?;
                Ok((format!("iks_d12_vertex({})", a.join(", ")), Ty::Void))
            }
            "dx12_end" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_d12_end()".to_string(), Ty::Void))
            }
            "dx12_swap" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_d12_swap()".to_string(), Ty::Int))
            }
            "dx12_title" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                let c = self.coerce(&c, &t, &Ty::Str, args[0].span())?;
                Ok((format!("iks_d12_title({})", c), Ty::Void))
            }
            "dx12_width" | "dx12_height" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = if name == "dx12_width" { "iks_d12_width" } else { "iks_d12_height" };
                Ok((format!("{}()", cfn), Ty::Int))
            }
            "dx12_rect" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_d12_rect({})", a.join(", ")), Ty::Void))
            }
            "dx12_circle" => {
                let a = self.float_args(&args[0..3], 3, name, span)?;
                let (sc, sty) = self.expr(&args[3])?;
                let sc = self.coerce(&sc, &sty, &Ty::Int, args[3].span())?;
                Ok((format!("iks_d12_circle({}, {}, {}, {})", a[0], a[1], a[2], sc), Ty::Void))
            }
            "dx12_line" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_d12_line({})", a.join(", ")), Ty::Void))
            }
            "d3d9_init" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (wc, wty) = self.expr(&args[0])?;
                let wc = self.coerce(&wc, &wty, &Ty::Int, args[0].span())?;
                let (hc, hty) = self.expr(&args[1])?;
                let hc = self.coerce(&hc, &hty, &Ty::Int, args[1].span())?;
                let (tcode, tty) = self.expr(&args[2])?;
                let tcode = self.coerce(&tcode, &tty, &Ty::Str, args[2].span())?;
                Ok((format!("iks_d9_init({}, {}, {})", wc, hc, tcode), Ty::Void))
            }
            "d3d9_clear" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_d9_clear({})", a.join(", ")), Ty::Void))
            }
            "d3d9_color" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_d9_color({})", a.join(", ")), Ty::Void))
            }
            "d3d9_begin" => {
                let a = self.int_args(args, 1, name, span)?;
                Ok((format!("iks_d9_begin({})", a[0]), Ty::Void))
            }
            "d3d9_vertex" => {
                let a = self.float_args(args, 3, name, span)?;
                Ok((format!("iks_d9_vertex({})", a.join(", ")), Ty::Void))
            }
            "d3d9_end" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_d9_end()".to_string(), Ty::Void))
            }
            "d3d9_swap" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_d9_swap()".to_string(), Ty::Int))
            }
            "d3d9_title" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                let c = self.coerce(&c, &t, &Ty::Str, args[0].span())?;
                Ok((format!("iks_d9_title({})", c), Ty::Void))
            }
            "d3d9_width" | "d3d9_height" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = if name == "d3d9_width" { "iks_d9_width" } else { "iks_d9_height" };
                Ok((format!("{}()", cfn), Ty::Int))
            }
            "d3d9_rect" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_d9_rect({})", a.join(", ")), Ty::Void))
            }
            "d3d9_circle" => {
                let a = self.float_args(&args[0..3], 3, name, span)?;
                let (sc, sty) = self.expr(&args[3])?;
                let sc = self.coerce(&sc, &sty, &Ty::Int, args[3].span())?;
                Ok((format!("iks_d9_circle({}, {}, {}, {})", a[0], a[1], a[2], sc), Ty::Void))
            }
            "d3d9_line" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_d9_line({})", a.join(", ")), Ty::Void))
            }
            "dx10_init" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (wc, wty) = self.expr(&args[0])?;
                let wc = self.coerce(&wc, &wty, &Ty::Int, args[0].span())?;
                let (hc, hty) = self.expr(&args[1])?;
                let hc = self.coerce(&hc, &hty, &Ty::Int, args[1].span())?;
                let (tcode, tty) = self.expr(&args[2])?;
                let tcode = self.coerce(&tcode, &tty, &Ty::Str, args[2].span())?;
                Ok((format!("iks_d10_init({}, {}, {})", wc, hc, tcode), Ty::Void))
            }
            "dx10_clear" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_d10_clear({})", a.join(", ")), Ty::Void))
            }
            "dx10_color" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_d10_color({})", a.join(", ")), Ty::Void))
            }
            "dx10_begin" => {
                let a = self.int_args(args, 1, name, span)?;
                Ok((format!("iks_d10_begin({})", a[0]), Ty::Void))
            }
            "dx10_vertex" => {
                let a = self.float_args(args, 3, name, span)?;
                Ok((format!("iks_d10_vertex({})", a.join(", ")), Ty::Void))
            }
            "dx10_end" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_d10_end()".to_string(), Ty::Void))
            }
            "dx10_swap" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_d10_swap()".to_string(), Ty::Int))
            }
            "dx10_title" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                let c = self.coerce(&c, &t, &Ty::Str, args[0].span())?;
                Ok((format!("iks_d10_title({})", c), Ty::Void))
            }
            "dx10_width" | "dx10_height" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = if name == "dx10_width" { "iks_d10_width" } else { "iks_d10_height" };
                Ok((format!("{}()", cfn), Ty::Int))
            }
            "dx10_rect" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_d10_rect({})", a.join(", ")), Ty::Void))
            }
            "dx10_circle" => {
                let a = self.float_args(&args[0..3], 3, name, span)?;
                let (sc, sty) = self.expr(&args[3])?;
                let sc = self.coerce(&sc, &sty, &Ty::Int, args[3].span())?;
                Ok((format!("iks_d10_circle({}, {}, {}, {})", a[0], a[1], a[2], sc), Ty::Void))
            }
            "dx10_line" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_d10_line({})", a.join(", ")), Ty::Void))
            }
            "webgpu_init" | "webgpu_clear" | "webgpu_color" | "webgpu_begin" | "webgpu_vertex" |
            "webgpu_end" | "webgpu_swap" | "webgpu_title" | "webgpu_width" | "webgpu_height" |
            "webgpu_rect" | "webgpu_circle" | "webgpu_line" => {
                Err(Diag::new(span, format!("'{}' is only available on the web target (--target=web)", name)))
            }
            "min" | "max" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (c1, t1) = self.expr(&args[0])?;
                let (c2, t2) = self.expr(&args[1])?;
                let both_int = t1 == Ty::Int && t2 == Ty::Int;
                let cfn = if name == "min" { "fmin" } else { "fmax" };
                let icfn = if name == "min" { "iks_imin" } else { "iks_imax" };
                if both_int {
                    Ok((format!("{}({}, {})", icfn, c1, c2), Ty::Int))
                } else {
                    let c1 = self.coerce(&c1, &t1, &Ty::Float, args[0].span())?;
                    let c2 = self.coerce(&c2, &t2, &Ty::Float, args[1].span())?;
                    Ok((format!("{}({}, {})", cfn, c1, c2), Ty::Float))
                }
            }
            "clamp" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (c1, t1) = self.expr(&args[0])?;
                let (c2, t2) = self.expr(&args[1])?;
                let (c3, t3) = self.expr(&args[2])?;
                let all_int = t1 == Ty::Int && t2 == Ty::Int && t3 == Ty::Int;
                if all_int {
                    Ok((format!("iks_iclamp({}, {}, {})", c1, c2, c3), Ty::Int))
                } else {
                    let c1 = self.coerce(&c1, &t1, &Ty::Float, args[0].span())?;
                    let c2 = self.coerce(&c2, &t2, &Ty::Float, args[1].span())?;
                    let c3 = self.coerce(&c3, &t3, &Ty::Float, args[2].span())?;
                    Ok((format!("fmin(fmax({}, {}), {})", c1, c2, c3), Ty::Float))
                }
            }
            "lerp" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (c1, t1) = self.expr(&args[0])?;
                let (c2, t2) = self.expr(&args[1])?;
                let (c3, t3) = self.expr(&args[2])?;
                if t1.is_vec() {
                    if t2 != t1 {
                        return Err(Diag::new(
                            args[1].span(),
                            format!("'lerp' expects '{}', found '{}'", t1, t2),
                        ));
                    }
                    let c3f = self.coerce(&c3, &t3, &Ty::Float, args[2].span())?;
                    let n = t1.vec_dim().unwrap();
                    return Ok((format!("iks_v{}_lerp({}, {}, {})", n, c1, c2, c3f), t1));
                }
                let c1 = self.coerce(&c1, &t1, &Ty::Float, args[0].span())?;
                let c2 = self.coerce(&c2, &t2, &Ty::Float, args[1].span())?;
                let c3 = self.coerce(&c3, &t3, &Ty::Float, args[2].span())?;
                Ok((format!("(({}) + (({}) - ({})) * ({}))", c1, c2, c1, c3), Ty::Float))
            }
            "vec2" | "vec3" | "vec4" => {
                let n: usize = match name {
                    "vec2" => 2,
                    "vec3" => 3,
                    _ => 4,
                };
                self.check_arity(name, args, n, n, span)?;
                let mut parts: Vec<String> = Vec::new();
                for a in args.iter() {
                    let (c, t) = self.expr(a)?;
                    parts.push(self.coerce(&c, &t, &Ty::Float, a.span())?);
                }
                let vt = Ty::vec_of_dim(n).unwrap();
                Ok((format!("(({}){{{}}})", c_ty(&vt), parts.join(", ")), vt))
            }
            "dot" | "length" | "distance" => {
                let nargs = if name == "length" { 1 } else { 2 };
                self.check_arity(name, args, nargs, nargs, span)?;
                let (c1, t1) = self.expr(&args[0])?;
                if !t1.is_vec() {
                    return Err(Diag::new(
                        args[0].span(),
                        format!("'{}' expects a vector, found '{}'", name, t1),
                    ));
                }
                let n = t1.vec_dim().unwrap();
                if name == "length" {
                    return Ok((format!("iks_v{}_len({})", n, c1), Ty::Float));
                }
                let (c2, t2) = self.expr(&args[1])?;
                if t2 != t1 {
                    return Err(Diag::new(
                        args[1].span(),
                        format!("'{}' expects '{}', found '{}'", name, t1, t2),
                    ));
                }
                if name == "dot" {
                    Ok((format!("iks_v{}_dot({}, {})", n, c1, c2), Ty::Float))
                } else {
                    Ok((
                        format!("iks_v{}_len(iks_v{}_sub({}, {}))", n, n, c1, c2),
                        Ty::Float,
                    ))
                }
            }
            "normalize" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                if !t.is_vec() {
                    return Err(Diag::new(
                        args[0].span(),
                        format!("'normalize' expects a vector, found '{}'", t),
                    ));
                }
                Ok((format!("iks_v{}_norm({})", t.vec_dim().unwrap(), c), t))
            }
            "cross" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (c1, t1) = self.expr(&args[0])?;
                let (c2, t2) = self.expr(&args[1])?;
                if t1 != Ty::Vec3 || t2 != Ty::Vec3 {
                    return Err(Diag::new(
                        span,
                        format!("'cross' expects two vec3 values, found '{}' and '{}'", t1, t2),
                    ));
                }
                Ok((format!("iks_v3_cross({}, {})", c1, c2), Ty::Vec3))
            }
            "rotate" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (c1, t1) = self.expr(&args[0])?;
                let (c2, t2) = self.expr(&args[1])?;
                if t1 != Ty::Vec2 {
                    return Err(Diag::new(
                        args[0].span(),
                        format!("'rotate' expects a vec2, found '{}'", t1),
                    ));
                }
                let c2f = self.coerce(&c2, &t2, &Ty::Float, args[1].span())?;
                Ok((format!("iks_v2_rot({}, {})", c1, c2f), Ty::Vec2))
            }
            "atan2" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (c1, t1) = self.expr(&args[0])?;
                let c1 = self.coerce(&c1, &t1, &Ty::Float, args[0].span())?;
                let (c2, t2) = self.expr(&args[1])?;
                let c2 = self.coerce(&c2, &t2, &Ty::Float, args[1].span())?;
                Ok((format!("atan2({}, {})", c1, c2), Ty::Float))
            }
            "round" | "ceil" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Float, args[0].span())?;
                Ok((format!("{}({})", name, code), Ty::Float))
            }
            "sign" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Float, args[0].span())?;
                Ok((format!("(({}) > 0 ? 1.0 : (({}) < 0 ? -1.0 : 0.0))", code, code), Ty::Float))
            }
            "hypot" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (c1, t1) = self.expr(&args[0])?;
                let c1 = self.coerce(&c1, &t1, &Ty::Float, args[0].span())?;
                let (c2, t2) = self.expr(&args[1])?;
                let c2 = self.coerce(&c2, &t2, &Ty::Float, args[1].span())?;
                Ok((format!("hypot({}, {})", c1, c2), Ty::Float))
            }
            "rand_range" => {
                self.check_arity(name, args, 2, 2, span)?;
                let a = self.int_args(args, 2, name, span)?;
                Ok((format!("iks_rand_range({}, {})", a[0], a[1]), Ty::Int))
            }
            "rand_int" => {
                self.check_arity(name, args, 2, 2, span)?;
                let a = self.int_args(args, 2, name, span)?;
                Ok((format!("iks_rand_range({}, {})", a[0], a[1]), Ty::Int))
            }
            "char_at" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (sc, sty) = self.expr(&args[0])?;
                let sc = self.coerce(&sc, &sty, &Ty::Str, args[0].span())?;
                let ic = self.int_args(&args[1..2], 1, name, span)?;
                Ok((format!("iks_char_at({}, {})", sc, ic[0]), Ty::Str))
            }
            "substr" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (sc, sty) = self.expr(&args[0])?;
                let sc = self.coerce(&sc, &sty, &Ty::Str, args[0].span())?;
                let a = self.int_args(&args[1..3], 2, name, span)?;
                Ok((format!("iks_substr({}, {}, {})", sc, a[0], a[1]), Ty::Str))
            }
            "str_find" => {
                self.check_arity(name, args, 2, 2, span)?;
                let a = self.str_args(args, 2, name, span)?;
                Ok((format!("iks_str_find({}, {})", a[0], a[1]), Ty::Int))
            }
            "str_replace" => {
                self.check_arity(name, args, 3, 3, span)?;
                let a = self.str_args(args, 3, name, span)?;
                Ok((format!("iks_str_replace({}, {}, {})", a[0], a[1], a[2]), Ty::Str))
            }
            "str_upper" | "str_lower" | "str_trim" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Str, args[0].span())?;
                let cfn = match name {
                    "str_upper" => "iks_str_upper",
                    "str_lower" => "iks_str_lower",
                    _ => "iks_str_trim",
                };
                Ok((format!("{}({})", cfn, code), Ty::Str))
            }
            "str_starts" | "str_ends" => {
                self.check_arity(name, args, 2, 2, span)?;
                let a = self.str_args(args, 2, name, span)?;
                let cfn = if name == "str_starts" { "iks_str_starts" } else { "iks_str_ends" };
                Ok((format!("{}({}, {})", cfn, a[0], a[1]), Ty::Bool))
            }
            "str_cmp" => {
                self.check_arity(name, args, 2, 2, span)?;
                let a = self.str_args(args, 2, name, span)?;
                Ok((format!("iks_str_cmp({}, {})", a[0], a[1]), Ty::Int))
            }
            "str_split" => {
                self.check_arity(name, args, 2, 2, span)?;
                let a = self.str_args(args, 2, name, span)?;
                Ok((format!("iks_str_split({}, {})", a[0], a[1]), Ty::Array(Box::new(Ty::Str))))
            }
            "str_join" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (ac, aty) = self.expr(&args[0])?;
                match &aty {
                    Ty::Array(e) if **e == Ty::Str => {
                        let (sc, sty) = self.expr(&args[1])?;
                        let sc = self.coerce(&sc, &sty, &Ty::Str, args[1].span())?;
                        Ok((format!("iks_str_join({}, {})", ac, sc), Ty::Str))
                    }
                    other => Err(Diag::new(
                        args[0].span(),
                        format!("'str_join' expects a string[], found '{}'", other),
                    )),
                }
            }
            "pop" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                match ty {
                    Ty::Array(_) => Ok((format!("iks_pop({})", code), ty)),
                    other => Err(Diag::new(
                        args[0].span(),
                        format!("'pop' expects an array, found '{}'", other),
                    )),
                }
            }
            "arr_remove" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let elem = match &ty {
                    Ty::Array(e) => (**e).clone(),
                    other => return Err(Diag::new(args[0].span(), format!("'arr_remove' expects an array, found '{}'", other))),
                };
                let ic = self.int_args(&args[1..2], 1, name, span)?;
                let _ = &elem;
                Ok((format!("iks_arr_remove({}, {})", code, ic[0]), ty))
            }
            "arr_insert" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let elem = match &ty {
                    Ty::Array(e) => (**e).clone(),
                    other => return Err(Diag::new(args[0].span(), format!("'arr_insert' expects an array, found '{}'", other))),
                };
                let ic = self.int_args(&args[1..2], 1, name, span)?;
                let (vcode, vty) = self.expr_hint(&args[2], Some(&elem))?;
                let vcode = self.coerce(&vcode, &vty, &elem, args[2].span())?;
                Ok((format!("iks_arr_insert({}, {}, &({}){{{}}})", code, ic[0], c_ty(&elem), vcode), ty))
            }
            "arr_slice" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (code, ty) = self.expr(&args[0])?;
                match ty {
                    Ty::Array(_) => {
                        let a = self.int_args(&args[1..3], 2, name, span)?;
                        Ok((format!("iks_arr_slice({}, {}, {})", code, a[0], a[1]), ty))
                    }
                    other => Err(Diag::new(args[0].span(), format!("'arr_slice' expects an array, found '{}'", other))),
                }
            }
            "arr_reverse" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                match ty {
                    Ty::Array(_) => Ok((format!("iks_arr_reverse({})", code), ty)),
                    other => Err(Diag::new(args[0].span(), format!("'arr_reverse' expects an array, found '{}'", other))),
                }
            }
            "arr_sort" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                match &ty {
                    Ty::Array(e) if **e == Ty::Int || **e == Ty::Float => {
                        Ok((format!("iks_arr_sort({})", code), ty))
                    }
                    Ty::Array(e) => Err(Diag::new(
                        args[0].span(),
                        format!("'arr_sort' expects int[] or float[], found '{}'", e),
                    )),
                    other => Err(Diag::new(args[0].span(), format!("'arr_sort' expects an array, found '{}'", other))),
                }
            }
            "arr_index_of" | "arr_contains" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let elem = match &ty {
                    Ty::Array(e) => (**e).clone(),
                    other => return Err(Diag::new(args[0].span(), format!("'{}' expects an array, found '{}'", name, other))),
                };
                let (vcode, vty) = self.expr_hint(&args[1], Some(&elem))?;
                let vcode = self.coerce(&vcode, &vty, &elem, args[1].span())?;
                let cfn = if name == "arr_index_of" { "iks_arr_index_of" } else { "iks_arr_contains" };
                let ret = if name == "arr_index_of" { Ty::Int } else { Ty::Bool };
                Ok((format!("{}({}, &({}){{{}}})", cfn, code, c_ty(&elem), vcode), ret))
            }
            "file_read" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Str, args[0].span())?;
                Ok((format!("iks_file_read({})", code), Ty::Str))
            }
            "file_write" => {
                self.check_arity(name, args, 2, 2, span)?;
                let a = self.str_args(args, 2, name, span)?;
                Ok((format!("iks_file_write({}, {})", a[0], a[1]), Ty::Bool))
            }
            "file_exists" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Str, args[0].span())?;
                Ok((format!("iks_file_exists({})", code), Ty::Bool))
            }
            "file_delete" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (code, ty) = self.expr(&args[0])?;
                let code = self.coerce(&code, &ty, &Ty::Str, args[0].span())?;
                Ok((format!("iks_file_delete({})", code), Ty::Bool))
            }
            "gfx_mouse_button" => {
                self.check_arity(name, args, 1, 1, span)?;
                let a = self.int_args(args, 1, name, span)?;
                Ok((format!("iks_gfx_mouse_button({})", a[0]), Ty::Bool))
            }
            "gl_rect" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_gl_rect({})", a.join(", ")), Ty::Void))
            }
            "gl_circle" => {
                let a = self.float_args(&args[0..3], 3, name, span)?;
                let (sc, sty) = self.expr(&args[3])?;
                let sc = self.coerce(&sc, &sty, &Ty::Int, args[3].span())?;
                Ok((format!("iks_gl_circle({}, {}, {}, {})", a[0], a[1], a[2], sc), Ty::Void))
            }
            "gl_line" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_gl_line({})", a.join(", ")), Ty::Void))
            }
            "vk_init" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (wc, wty) = self.expr(&args[0])?;
                let wc = self.coerce(&wc, &wty, &Ty::Int, args[0].span())?;
                let (hc, hty) = self.expr(&args[1])?;
                let hc = self.coerce(&hc, &hty, &Ty::Int, args[1].span())?;
                let (tcode, tty) = self.expr(&args[2])?;
                let tcode = self.coerce(&tcode, &tty, &Ty::Str, args[2].span())?;
                Ok((format!("iks_vk_init({}, {}, {})", wc, hc, tcode), Ty::Void))
            }
            "vk_clear" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_vk_clear({})", a.join(", ")), Ty::Void))
            }
            "vk_color" => {
                let a = self.int_args(args, 3, name, span)?;
                Ok((format!("iks_vk_color({})", a.join(", ")), Ty::Void))
            }
            "vk_begin" => {
                let a = self.int_args(args, 1, name, span)?;
                Ok((format!("iks_vk_begin({})", a[0]), Ty::Void))
            }
            "vk_vertex" => {
                let a = self.float_args(args, 3, name, span)?;
                Ok((format!("iks_vk_vertex({})", a.join(", ")), Ty::Void))
            }
            "vk_end" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_vk_end()".to_string(), Ty::Void))
            }
            "vk_swap" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_vk_swap()".to_string(), Ty::Int))
            }
            "vk_title" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (c, t) = self.expr(&args[0])?;
                let c = self.coerce(&c, &t, &Ty::Str, args[0].span())?;
                Ok((format!("iks_vk_title({})", c), Ty::Void))
            }
            "vk_width" | "vk_height" => {
                self.check_arity(name, args, 0, 0, span)?;
                let cfn = if name == "vk_width" { "iks_vk_width" } else { "iks_vk_height" };
                Ok((format!("{}()", cfn), Ty::Int))
            }
            "vk_rect" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_vk_rect({})", a.join(", ")), Ty::Void))
            }
            "vk_circle" => {
                let a = self.float_args(&args[0..3], 3, name, span)?;
                let (sc, sty) = self.expr(&args[3])?;
                let sc = self.coerce(&sc, &sty, &Ty::Int, args[3].span())?;
                Ok((format!("iks_vk_circle({}, {}, {}, {})", a[0], a[1], a[2], sc), Ty::Void))
            }
            "vk_line" => {
                let a = self.float_args(args, 4, name, span)?;
                Ok((format!("iks_vk_line({})", a.join(", ")), Ty::Void))
            }
            "map_new" => {
                self.check_arity(name, args, 0, 0, span)?;
                Ok(("iks_map_new()".to_string(), Ty::HashMap))
            }
            "map_set" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (mc, mty) = self.expr(&args[0])?;
                if mty != Ty::HashMap {
                    return Err(Diag::new(args[0].span(), format!("'map_set' expects a map, found '{}'", mty)));
                }
                let (kc, kty) = self.expr(&args[1])?;
                let kc = self.coerce(&kc, &kty, &Ty::Str, args[1].span())?;
                let (vc, vty) = self.expr(&args[2])?;
                let cfn = match vty {
                    Ty::Int => "iks_map_set_i",
                    Ty::Float => "iks_map_set_f",
                    Ty::Bool => "iks_map_set_b",
                    Ty::Str => "iks_map_set_s",
                    other => return Err(Diag::new(args[2].span(), format!("map value must be int, float, bool or string, found '{}'", other))),
                };
                Ok((format!("{}({}, {}, {})", cfn, mc, kc, vc), Ty::Void))
            }
            "map_get" => {
                self.check_arity(name, args, 3, 3, span)?;
                let (mc, mty) = self.expr(&args[0])?;
                if mty != Ty::HashMap {
                    return Err(Diag::new(args[0].span(), format!("'map_get' expects a map, found '{}'", mty)));
                }
                let (kc, kty) = self.expr(&args[1])?;
                let kc = self.coerce(&kc, &kty, &Ty::Str, args[1].span())?;
                let (dc, dty) = self.expr(&args[2])?;
                let cfn = match dty {
                    Ty::Int => "iks_map_get_i",
                    Ty::Float => "iks_map_get_f",
                    Ty::Bool => "iks_map_get_b",
                    Ty::Str => "iks_map_get_s",
                    other => return Err(Diag::new(args[2].span(), format!("map default must be int, float, bool or string, found '{}'", other))),
                };
                Ok((format!("{}({}, {}, {})", cfn, mc, kc, dc), dty))
            }
            "map_has" | "map_remove" => {
                self.check_arity(name, args, 2, 2, span)?;
                let (mc, mty) = self.expr(&args[0])?;
                if mty != Ty::HashMap {
                    return Err(Diag::new(args[0].span(), format!("'{}' expects a map, found '{}'", name, mty)));
                }
                let (kc, kty) = self.expr(&args[1])?;
                let kc = self.coerce(&kc, &kty, &Ty::Str, args[1].span())?;
                let cfn = if name == "map_has" { "iks_map_has" } else { "iks_map_remove" };
                Ok((format!("{}({}, {})", cfn, mc, kc), Ty::Bool))
            }
            "map_keys" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (mc, mty) = self.expr(&args[0])?;
                if mty != Ty::HashMap {
                    return Err(Diag::new(args[0].span(), format!("'map_keys' expects a map, found '{}'", mty)));
                }
                Ok((format!("iks_map_keys({})", mc), Ty::Array(Box::new(Ty::Str))))
            }
            "map_len" => {
                self.check_arity(name, args, 1, 1, span)?;
                let (mc, mty) = self.expr(&args[0])?;
                if mty != Ty::HashMap {
                    return Err(Diag::new(args[0].span(), format!("'map_len' expects a map, found '{}'", mty)));
                }
                Ok((format!("iks_map_len({})", mc), Ty::Int))
            }
            _ => {
                if let Some(v) = self.variants.get(name).cloned() {
                    return self.variant_ctor(name, args, span, &v);
                }
                let Some(sig) = self.sigs.get(name).cloned() else {
                    return Err(Diag::new(span, format!("unknown function '{}'", name)));
                };
                self.check_arity(name, args, sig.params.len(), sig.params.len(), span)?;
                let mut parts = Vec::new();
                for (arg, pty) in args.iter().zip(sig.params.iter()) {
                    let (code, ty) = self.expr_hint(arg, Some(pty))?;
                    let code = self.coerce(&code, &ty, pty, arg.span())?;
                    parts.push(code);
                }
                Ok((
                    format!("{}({})", c_name(name, sig.is_extern), parts.join(", ")),
                    sig.ret.clone(),
                ))
            }
        }
    }

    fn ensure_tuple_typedef(&mut self, ty: &Ty, span: Span) -> Result<(), Diag> {
        let Ty::Tuple(ts) = ty else {
            return Err(Diag::new(span, "internal: not a tuple type"));
        };
        let cstruct = c_ty(ty);
        if self.tuple_seen.contains(&cstruct) {
            return Ok(());
        }
        self.tuple_seen.insert(cstruct.clone());
        let mut def = format!("typedef struct {} {{\n", cstruct);
        for (i, t) in ts.iter().enumerate() {
            def.push_str(&format!("    {} _{};\n", c_ty(t), i));
        }
        def.push_str(&format!("}} {};\n", cstruct));
        self.tuple_defs.push_str(&def);
        Ok(())
    }

    fn check_arity(
        &self,
        name: &str,
        args: &[Expr],
        min: usize,
        max: usize,
        span: Span,
    ) -> Result<(), Diag> {
        if args.len() < min || args.len() > max {
            let expected = if min == max {
                format!("{}", min)
            } else {
                format!("{} to {}", min, max)
            };
            return Err(Diag::new(
                span,
                format!("'{}' expects {} argument(s), got {}", name, expected, args.len()),
            ));
        }
        Ok(())
    }
}

fn unify_elems(tys: &[Ty], span: Span) -> Result<Ty, Diag> {
    let Some(first) = tys.first() else {
        return Err(Diag::new(span, "empty array literal"));
    };
    let mut result = first.clone();
    for t in &tys[1..] {
        if result == *t {
            continue;
        }
        if result.is_numeric() && t.is_numeric() {
            result = Ty::Float;
            continue;
        }
        return Err(Diag::new(
            span,
            format!("mixed element types in array literal: '{}' and '{}'", result, t),
        ));
    }
    Ok(result)
}

fn block_terminates(b: &Block, exhaustive: &[Span]) -> bool {
    b.stmts.iter().any(|s| stmt_terminates(s, exhaustive))
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// Pattern bindings, or an empty list for value/bare patterns.
pub(crate) fn pattern_bindings(p: &Pattern) -> &[String] {
    match p {
        Pattern::Variant { bindings, .. } => bindings,
        _ => &[],
    }
}

/// Render a non-enum pattern back into an expression so ordinary value
/// switches keep working unchanged.
pub(crate) fn as_value_pattern(p: &Pattern) -> Expr {
    match p {
        Pattern::Value(e) => e.clone(),
        Pattern::Bare { name, span } => Expr::Var(name.clone(), *span),
        Pattern::Variant { name, bindings, span } => Expr::Call {
            name: name.clone(),
            args: bindings.iter().map(|b| Expr::Var(b.clone(), *span)).collect(),
            span: *span,
        },
    }
}

/// True when control flow definitely leaves the current block after this
/// statement, making anything that follows it unreachable.
fn stmt_hard_terminates(s: &Stmt) -> bool {
    match s {
        Stmt::Return { .. } | Stmt::Break(_) | Stmt::Continue(_) => true,
        Stmt::Block(b) => b.stmts.iter().any(stmt_hard_terminates),
        _ => false,
    }
}

fn stmt_terminates(s: &Stmt, exhaustive: &[Span]) -> bool {
    match s {
        Stmt::Return { .. } => true,
        Stmt::If { then, els, .. } => match els {
            Some(e) => block_terminates(then, exhaustive) && stmt_terminates(e, exhaustive),
            None => false,
        },
        Stmt::Switch { cases, default, span, .. } => match default {
            Some(d) => {
                block_terminates(d, exhaustive) && cases.iter().all(|c| block_terminates(&c.body, exhaustive))
            }
            None => {
                exhaustive.contains(span)
                    && cases.iter().all(|c| block_terminates(&c.body, exhaustive))
            }
        },
        Stmt::Block(b) => block_terminates(b, exhaustive),
        _ => false,
    }
}

fn escape_c(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            other => out.push(other),
        }
    }
    out
}
