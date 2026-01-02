#include "reml/runtime/enum.h"

#include <stdlib.h>
#include <string.h>

reml_enum_value *reml_enum_make(int32_t tag, size_t payload_size) {
  reml_enum_value *value = (reml_enum_value *)malloc(sizeof(reml_enum_value));
  if (!value) {
    return NULL;
  }
  value->tag = tag;
  value->payload = NULL;
  if (payload_size > 0) {
    value->payload = malloc(payload_size);
    if (!value->payload) {
      free(value);
      return NULL;
    }
    memset(value->payload, 0, payload_size);
  }
  return value;
}

void reml_enum_free(reml_enum_value *value) {
  if (!value) {
    return;
  }
  free(value->payload);
  free(value);
}

void *reml_enum_payload(reml_enum_value *value) {
  return value ? value->payload : NULL;
}
