#ifndef REML_TEXT_GRAPHEME_H
#define REML_TEXT_GRAPHEME_H

#include <stddef.h>

#include "reml/text/unicode.h"
#include "reml/util/string_view.h"

#ifdef __cplusplus
extern "C" {
#endif

size_t reml_grapheme_advance(const char *input, size_t length, size_t offset,
                             reml_unicode_error *out_error);
size_t reml_grapheme_len(reml_string_view view);
size_t reml_grapheme_display_width(reml_string_view view);
bool reml_grapheme_slice(reml_string_view view, size_t start, size_t end, reml_string_view *out);

#ifdef __cplusplus
}
#endif

#endif
