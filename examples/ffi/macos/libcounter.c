// 簡易カウンタのデモ用 C ライブラリ。
// Reml 側から所有権移転付きでハンドルを受け取り、インクリメントと解放を行う。
#include <stdint.h>
#include <stdlib.h>

typedef struct {
  int32_t value;
} FfiCounter;

__attribute__((visibility("default")))
FfiCounter *ffi_counter_new(int32_t initial) {
  FfiCounter *counter = (FfiCounter *)malloc(sizeof(FfiCounter));
  if (counter == NULL) {
    return NULL;
  }
  counter->value = initial;
  return counter;
}

__attribute__((visibility("default")))
void ffi_counter_increment(FfiCounter *counter, int32_t delta) {
  if (counter == NULL) {
    return;
  }
  counter->value += delta;
}

__attribute__((visibility("default")))
int32_t ffi_counter_get(const FfiCounter *counter) {
  if (counter == NULL) {
    return -1;
  }
  return counter->value;
}

__attribute__((visibility("default")))
void ffi_counter_free(FfiCounter *counter) {
  free(counter);
}
