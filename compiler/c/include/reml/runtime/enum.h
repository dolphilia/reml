#ifndef REML_RUNTIME_ENUM_H
#define REML_RUNTIME_ENUM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  int32_t tag;
  void *payload;
} reml_enum_value;

reml_enum_value *reml_enum_make(int32_t tag, size_t payload_size);
void reml_enum_free(reml_enum_value *value);
void *reml_enum_payload(reml_enum_value *value);

#ifdef __cplusplus
}
#endif

#endif
