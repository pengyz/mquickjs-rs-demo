/* This file contains a DELIBERATE syntax error.
 *
 * Phase 1 acceptance: the mquickjs parser unwinds parse errors through
 * setjmp/longjmp (mquickjs.c) — this case must surface as
 * `CASE demo_syntax_error.js FAIL: ...` on the sentinel protocol,
 * NOT crash the sim image.
 */

function broken( {
  return 1;
}
