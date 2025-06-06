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
const int32_t x = 42    ;
const uint8_t y = 255    ;
const int64_t z = (-1000000)    ;
const lang_float pi = 3.14159f    ;
const bool flag = true    ;
const bool other = false    ;
const int32_t sum = (x + 10)    ;
const int32_t diff = (x - 5)    ;
const int32_t prod = (x * 2)    ;
const int32_t quot = (x / 3)    ;
const int32_t rem = (x % 10)    ;
const uint32_t a = 65280    ;
const uint32_t b = 255    ;
const uint32_t bit_and = (a & b)    ;
const uint32_t bit_or = (a | b)    ;
const uint32_t bit_xor = (a ^ b)    ;
const uint32_t bit_not = (~a)    ;
const uint32_t left = (a << 2)    ;
const uint32_t right = (a >> 2)    ;
const bool eq = (x == 42)    ;
const bool neq = (x != 0)    ;
const bool lt = (x < 100)    ;
const bool gt = (x > 0)    ;
const bool lte = (x <= 42)    ;
const bool gte = (x >= 42)    ;
const bool and_result = (flag && other)    ;
const bool or_result = (flag || other)    ;
const bool not_result = (!flag)    ;
const uint8_t byte_val = ((uint8_t)x)    ;
const lang_float float_val = ((lang_float)x)    ;
return 0    ;
}
