#ifndef REML_RUNTIME_STRING_H
#define REML_RUNTIME_STRING_H

#include <stddef.h>
#include <stdint.h>

#include "reml/text/string.h"

#ifdef __cplusplus
extern "C" {
#endif

reml_string *reml_string_from_utf8(const char *data, size_t len);
reml_string *reml_string_concat(const reml_string *left, const reml_string *right);
int32_t reml_string_cmp(const reml_string *left, const reml_string *right);
void reml_string_free(reml_string *str);

#ifdef __cplusplus
}
#endif

#endif
