#ifndef REML_MANIFEST_H
#define REML_MANIFEST_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  char *package_name;
  char *package_version;
} reml_manifest;

typedef struct {
  const char *message;
} reml_manifest_error;

bool reml_manifest_load(const char *path, reml_manifest *out, reml_manifest_error *error);
void reml_manifest_free(reml_manifest *manifest);

#ifdef __cplusplus
}
#endif

#endif
