#include "reml/manifest/manifest.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <toml.h>

static char *reml_read_file(const char *path, size_t *out_length) {
  FILE *fp = fopen(path, "rb");
  if (!fp) {
    return NULL;
  }
  if (fseek(fp, 0, SEEK_END) != 0) {
    fclose(fp);
    return NULL;
  }
  long size = ftell(fp);
  if (size < 0) {
    fclose(fp);
    return NULL;
  }
  if (fseek(fp, 0, SEEK_SET) != 0) {
    fclose(fp);
    return NULL;
  }
  char *buffer = (char *)malloc((size_t)size + 1);
  if (!buffer) {
    fclose(fp);
    return NULL;
  }
  size_t read_bytes = fread(buffer, 1, (size_t)size, fp);
  fclose(fp);
  if (read_bytes != (size_t)size) {
    free(buffer);
    return NULL;
  }
  buffer[size] = '\0';
  if (out_length) {
    *out_length = (size_t)size;
  }
  return buffer;
}

static void reml_manifest_clear(reml_manifest *manifest) {
  if (!manifest) {
    return;
  }
  manifest->package_name = NULL;
  manifest->package_version = NULL;
}

bool reml_manifest_load(const char *path, reml_manifest *out, reml_manifest_error *error) {
  if (!out) {
    if (error) {
      error->message = "manifest output is null";
    }
    return false;
  }

  reml_manifest_clear(out);

  size_t length = 0;
  char *content = reml_read_file(path, &length);
  if (!content) {
    if (error) {
      error->message = "failed to read manifest";
    }
    return false;
  }

  char errbuf[200];
  toml_table_t *root = toml_parse(content, errbuf, sizeof(errbuf));
  if (!root) {
    if (error) {
      error->message = "invalid toml";
    }
    free(content);
    return false;
  }

  toml_table_t *package = toml_table_in(root, "package");
  if (!package) {
    if (error) {
      error->message = "missing [package] section";
    }
    toml_free(root);
    free(content);
    return false;
  }

  toml_datum_t name = toml_string_in(package, "name");
  if (!name.ok) {
    if (error) {
      error->message = "missing package.name";
    }
    toml_free(root);
    free(content);
    return false;
  }

  toml_datum_t version = toml_string_in(package, "version");
  if (!version.ok) {
    if (error) {
      error->message = "missing package.version";
    }
    free(name.u.s);
    toml_free(root);
    free(content);
    return false;
  }

  out->package_name = name.u.s;
  out->package_version = version.u.s;

  toml_free(root);
  free(content);
  return true;
}

void reml_manifest_free(reml_manifest *manifest) {
  if (!manifest) {
    return;
  }
  free(manifest->package_name);
  free(manifest->package_version);
  reml_manifest_clear(manifest);
}
