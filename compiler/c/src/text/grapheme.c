#include "reml/text/grapheme.h"

#include <stdlib.h>

#include <utf8proc.h>

static bool reml_is_surrogate_sequence(const char *input, size_t length, size_t offset) {
  if (length - offset < 3) {
    return false;
  }
  unsigned char b0 = (unsigned char)input[offset];
  unsigned char b1 = (unsigned char)input[offset + 1];
  unsigned char b2 = (unsigned char)input[offset + 2];
  return b0 == 0xED && (b1 & 0xE0) == 0xA0 && (b2 & 0xC0) == 0x80;
}

static void reml_grapheme_set_error(reml_unicode_error *out_error, reml_unicode_error_kind kind,
                                    size_t offset, size_t length) {
  if (!out_error) {
    return;
  }
  out_error->kind = kind;
  out_error->offset = offset;
  out_error->length = length;
}

size_t reml_grapheme_advance(const char *input, size_t length, size_t offset,
                             reml_unicode_error *out_error) {
  if (offset >= length) {
    reml_grapheme_set_error(out_error, REML_UNICODE_OK, 0, 0);
    return 0;
  }

  size_t index = offset;
  utf8proc_int32_t state = 0;
  utf8proc_int32_t prev = 0;
  bool has_prev = false;

  while (index < length) {
    utf8proc_int32_t codepoint = 0;
    utf8proc_ssize_t len =
        utf8proc_iterate((const utf8proc_uint8_t *)input + index, (utf8proc_ssize_t)(length - index),
                         &codepoint);
    if (len < 0) {
      if (reml_is_surrogate_sequence(input, length, index)) {
        reml_grapheme_set_error(out_error, REML_UNICODE_INVALID_SCALAR, index, 3);
      } else {
        reml_grapheme_set_error(out_error, REML_UNICODE_INVALID_UTF8, index, 1);
      }
      return 0;
    }
    if (!utf8proc_codepoint_valid(codepoint)) {
      reml_grapheme_set_error(out_error, REML_UNICODE_INVALID_SCALAR, index, (size_t)len);
      return 0;
    }

    if (!has_prev) {
      has_prev = true;
      prev = codepoint;
      index += (size_t)len;
      continue;
    }

    if (utf8proc_grapheme_break_stateful(prev, codepoint, &state)) {
      break;
    }

    prev = codepoint;
    index += (size_t)len;
  }

  reml_grapheme_set_error(out_error, REML_UNICODE_OK, 0, 0);
  return index - offset;
}

static size_t reml_grapheme_width(const char *input, size_t length, size_t offset,
                                  size_t *out_advance, reml_unicode_error *out_error) {
  size_t index = offset;
  utf8proc_int32_t state = 0;
  utf8proc_int32_t prev = 0;
  bool has_prev = false;
  size_t width = 0;
  bool has_zwj = false;
  size_t regional_indicators = 0;

  while (index < length) {
    utf8proc_int32_t codepoint = 0;
    utf8proc_ssize_t len =
        utf8proc_iterate((const utf8proc_uint8_t *)input + index, (utf8proc_ssize_t)(length - index),
                         &codepoint);
    if (len < 0) {
      if (reml_is_surrogate_sequence(input, length, index)) {
        reml_grapheme_set_error(out_error, REML_UNICODE_INVALID_SCALAR, index, 3);
      } else {
        reml_grapheme_set_error(out_error, REML_UNICODE_INVALID_UTF8, index, 1);
      }
      return 0;
    }
    if (!utf8proc_codepoint_valid(codepoint)) {
      reml_grapheme_set_error(out_error, REML_UNICODE_INVALID_SCALAR, index, (size_t)len);
      return 0;
    }

    if (has_prev && utf8proc_grapheme_break_stateful(prev, codepoint, &state)) {
      break;
    }

    if (codepoint == 0x200D) {
      has_zwj = true;
    }
    if (codepoint >= 0x1F1E6 && codepoint <= 0x1F1FF) {
      regional_indicators += 1;
    }

    int char_width = utf8proc_charwidth(codepoint);
    if (char_width > (int)width) {
      width = (size_t)char_width;
    }

    has_prev = true;
    prev = codepoint;
    index += (size_t)len;
  }

  if (has_zwj) {
    width = 2;
  } else if (regional_indicators >= 2) {
    width = 2;
  }

  reml_grapheme_set_error(out_error, REML_UNICODE_OK, 0, 0);
  if (out_advance) {
    *out_advance = index - offset;
  }
  return width;
}

size_t reml_grapheme_len(reml_string_view view) {
  size_t offset = 0;
  size_t count = 0;
  reml_unicode_error error;

  while (offset < view.length) {
    size_t advance = reml_grapheme_advance(view.data, view.length, offset, &error);
    if (advance == 0) {
      return 0;
    }
    offset += advance;
    count += 1;
  }

  return count;
}

size_t reml_grapheme_display_width(reml_string_view view) {
  size_t offset = 0;
  size_t width = 0;
  reml_unicode_error error;

  while (offset < view.length) {
    size_t advance = 0;
    size_t grapheme_width = reml_grapheme_width(view.data, view.length, offset, &advance, &error);
    if (advance == 0) {
      return 0;
    }
    width += grapheme_width;
    offset += advance;
  }

  return width;
}

bool reml_grapheme_slice(reml_string_view view, size_t start, size_t end, reml_string_view *out) {
  size_t offset = 0;
  size_t index = 0;
  reml_unicode_error error;
  size_t start_offset = 0;
  size_t end_offset = 0;

  while (offset < view.length && index < end) {
    size_t advance = reml_grapheme_advance(view.data, view.length, offset, &error);
    if (advance == 0) {
      return false;
    }
    if (index == start) {
      start_offset = offset;
    }
    offset += advance;
    index += 1;
    if (index == end) {
      end_offset = offset;
      break;
    }
  }

  if (start > end || index < end) {
    return false;
  }

  if (start == end) {
    start_offset = offset;
    end_offset = offset;
  }

  if (out) {
    out->data = view.data + start_offset;
    out->length = end_offset - start_offset;
  }
  return true;
}
