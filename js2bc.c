#include "quickjs.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
  if (argc != 3) {
    fprintf(stderr, "Usage: js2bc <input.js> <output.bc>\n");
    return 1;
  }

  const char *in_filename = argv[1];
  const char *out_filename = argv[2];

  FILE *f_in = fopen(in_filename, "rb");
  if (!f_in) {
    perror("fopen input");
    return 1;
  }

  fseek(f_in, 0, SEEK_END);
  long size = ftell(f_in);
  fseek(f_in, 0, SEEK_SET);

  char *source = malloc(size + 1);
  if (!source) {
    perror("malloc");
    return 1;
  }

  // Read the file and ensure it is null-terminated
  size_t read_bytes = fread(source, 1, size, f_in);
  source[read_bytes] = '\0';
  fclose(f_in);

  JSRuntime *rt = JS_NewRuntime();
  JSContext *ctx = JS_NewContext(rt);

  // Compile the source into QuickJS bytecode (global script mode)
  // using JS_EVAL_FLAG_COMPILE_ONLY.
  JSValue obj = JS_Eval(ctx, source, read_bytes, "<pintora>",
                        JS_EVAL_TYPE_GLOBAL | JS_EVAL_FLAG_COMPILE_ONLY);

  free(source);

  if (JS_IsException(obj)) {
    JSValue exception = JS_GetException(ctx);
    const char *err_msg = JS_ToCString(ctx, exception);
    fprintf(stderr, "JS Compilation Error: %s\n",
            err_msg ? err_msg : "unknown");
    JS_FreeCString(ctx, err_msg);
    JS_FreeValue(ctx, exception);
    JS_FreeValue(ctx, obj);
    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);
    return 1;
  }

  // Write the compiled bytecode to a buffer
  size_t out_size;
  uint8_t *out_buf = JS_WriteObject(ctx, &out_size, obj, JS_WRITE_OBJ_BYTECODE);
  JS_FreeValue(ctx, obj);

  if (!out_buf) {
    fprintf(stderr, "JS_WriteObject failed\n");
    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);
    return 1;
  }

  // Save the bytecode
  FILE *f_out = fopen(out_filename, "wb");
  if (!f_out) {
    perror("fopen output");
    return 1;
  }

  fwrite(out_buf, 1, out_size, f_out);
  fclose(f_out);

  js_free(ctx, out_buf);
  JS_FreeContext(ctx);
  JS_FreeRuntime(rt);

  return 0;
}
