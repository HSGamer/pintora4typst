// Minimal TextEncoder/TextDecoder polyfill (UTF-8 only)
// Replaces the full encoding.js + encoding-indexes.js (~630KB)
// Pintora only uses UTF-8 and ASCII encodings.

(function(global) {
  'use strict';

  if (typeof global.TextEncoder === 'undefined') {
    global.TextEncoder = function TextEncoder() {
      this.encoding = 'utf-8';
    };
    global.TextEncoder.prototype.encode = function(str) {
      var bytes = [];
      for (var i = 0; i < str.length; i++) {
        var c = str.charCodeAt(i);
        if (c < 0x80) {
          bytes.push(c);
        } else if (c < 0x800) {
          bytes.push(0xC0 | (c >> 6), 0x80 | (c & 0x3F));
        } else if (c >= 0xD800 && c <= 0xDBFF && i + 1 < str.length) {
          var next = str.charCodeAt(i + 1);
          if (next >= 0xDC00 && next <= 0xDFFF) {
            var cp = ((c - 0xD800) << 10) + (next - 0xDC00) + 0x10000;
            bytes.push(0xF0 | (cp >> 18), 0x80 | ((cp >> 12) & 0x3F), 0x80 | ((cp >> 6) & 0x3F), 0x80 | (cp & 0x3F));
            i++;
          }
        } else if (c < 0x10000) {
          bytes.push(0xE0 | (c >> 12), 0x80 | ((c >> 6) & 0x3F), 0x80 | (c & 0x3F));
        }
      }
      return new Uint8Array(bytes);
    };
  }

  if (typeof global.TextDecoder === 'undefined') {
    global.TextDecoder = function TextDecoder(encoding) {
      this.encoding = (encoding || 'utf-8').toLowerCase();
    };
    global.TextDecoder.prototype.decode = function(bytes) {
      if (!bytes) return '';
      var arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
      // For ASCII encoding, just map bytes to chars
      if (this.encoding === 'ascii' || this.encoding === 'us-ascii') {
        var result = '';
        for (var i = 0; i < arr.length; i++) {
          result += String.fromCharCode(arr[i] & 0x7F);
        }
        return result;
      }
      // UTF-8 decoding
      var result = '';
      for (var i = 0; i < arr.length; ) {
        var b = arr[i];
        if (b < 0x80) {
          result += String.fromCharCode(b);
          i++;
        } else if ((b & 0xE0) === 0xC0) {
          result += String.fromCharCode(((b & 0x1F) << 6) | (arr[i+1] & 0x3F));
          i += 2;
        } else if ((b & 0xF0) === 0xE0) {
          result += String.fromCharCode(((b & 0x0F) << 12) | ((arr[i+1] & 0x3F) << 6) | (arr[i+2] & 0x3F));
          i += 3;
        } else if ((b & 0xF8) === 0xF0) {
          var cp = ((b & 0x07) << 18) | ((arr[i+1] & 0x3F) << 12) | ((arr[i+2] & 0x3F) << 6) | (arr[i+3] & 0x3F);
          cp -= 0x10000;
          result += String.fromCharCode(0xD800 + (cp >> 10), 0xDC00 + (cp & 0x3FF));
          i += 4;
        } else {
          i++;
        }
      }
      return result;
    };
  }
})(typeof globalThis !== 'undefined' ? globalThis : typeof self !== 'undefined' ? self : this);