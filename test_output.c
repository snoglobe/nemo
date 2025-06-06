#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#if defined(__LP64__) || defined(_WIN64)
typedef int64_t lang_int;
typedef uint64_t lang_uint;
#else
typedef int32_t lang_int;
typedef uint32_t lang_uint;
#endif

typedef float lang_float;

typedef struct {} lang_nil;

typedef struct {
    const uint8_t* data;
    lang_int len;
} lang_str;


int32_t main(void);

int32_t main(void) {
    const     lang_float pi =     3.14f    ;
    return         0    ;
}
