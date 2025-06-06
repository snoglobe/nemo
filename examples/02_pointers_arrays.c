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
const const int32_t* px = (&x)    ;
const int32_t val = (*px)    ;
int32_t y = 100    ;
const int32_t* py = (&y)    ;
((*py) = 200)    ;
const int32_t[] arr = {10, 20, 30, 40, 50}    ;
const int32_t first = arr[0]    ;
const int32_t last = arr[4]    ;
int32_t[] marr = {1, 2, 3}    ;
(marr[1] = 99)    ;
const const int32_t* p = (&arr[0])    ;
const const int32_t* p2 = (p + 2)    ;
const int32_t val2 = (*p2)    ;
const lang_int diff = (p2 - p)    ;
int32_t sum = 0    ;
for (int _i = 0; _i < /* array size */; _i++)     {
(sum += n)        ;
}    
for (int _i = 0; _i < /* array size */; _i++)     {
((*p) = ((*p) * 2))        ;
}    
const const int32_t* null_ptr = ((const int32_t*)0)    ;
const const const int32_t** ppx = (&px)    ;
const int32_t val3 = (*(*ppx))    ;
const const int32_t*[] parr = {(&arr[0]), (&arr[1]), (&arr[2])}    ;
return 0    ;
}
