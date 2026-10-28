//*************************************************************************
// JANUS is a simple, robust, open standard signalling method for         *
// underwater communications. See <http://www.januswiki.org> for details. *
//*************************************************************************
// Example software implementations provided by STO CMRE are subject to   *
// Copyright (C) 2008-2018 STO Centre for Maritime Research and           *
// Experimentation (CMRE)                                                 *
//                                                                        *
// This is free software: you can redistribute it and/or modify it        *
// under the terms of the GNU General Public License version 3 as         *
// published by the Free Software Foundation.                             *
//                                                                        *
// This program is distributed in the hope that it will be useful, but    *
// WITHOUT ANY WARRANTY; without even the implied warranty of FITNESS     *
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for       *
// more details.                                                          *
//                                                                        *
// You should have received a copy of the GNU General Public License      *
// along with this program. If not, see <http://www.gnu.org/licenses/>.   *
//*************************************************************************
// Authors: Ricardo Martins, Giovanni Zappa, Luigi Elia D'Amaro           *
//*************************************************************************

//*************************************************************************
// JANUS is a simple, robust, open standard signalling method for         *
// underwater communications. See <http://www.januswiki.org> for details. *
//*************************************************************************
// Example software implementations provided by STO CMRE are subject to   *
// Copyright (C) 2008-2018 STO Centre for Maritime Research and           *
// Experimentation (CMRE)                                                 *
//                                                                        *
// This is free software: you can redistribute it and/or modify it        *
// under the terms of the GNU General Public License version 3 as         *
// published by the Free Software Foundation.                             *
//                                                                        *
// This program is distributed in the hope that it will be useful, but    *
// WITHOUT ANY WARRANTY; without even the implied warranty of FITNESS     *
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for       *
// more details.                                                          *
//                                                                        *
// You should have received a copy of the GNU General Public License      *
// along with this program. If not, see <http://www.gnu.org/licenses/>.   *
//*************************************************************************
// Author: Ricardo Martins                                                *
//*************************************************************************

#ifndef JANUS_EXPORT_H_INCLUDED_
#define JANUS_EXPORT_H_INCLUDED_

#ifdef __cplusplus
#define JANUS_EXPORT extern "C"
#if (defined(_WIN32) || defined(__WIN32__))
#define JANUS_PLUGIN_EXPORT extern "C" __declspec(dllexport)
#else
#define JANUS_PLUGIN_EXPORT extern "C"
#endif
#else
#define JANUS_EXPORT
#if (defined(_WIN32) || defined(__WIN32__))
#define JANUS_PLUGIN_EXPORT __declspec(dllexport)
#else
#define JANUS_PLUGIN_EXPORT
#endif
#endif

#ifdef _MSC_VER
#ifndef inline
#define inline __inline
#endif
#endif

#endif

#ifndef JANUS_TYPES_H_INCLUDED_
#define JANUS_TYPES_H_INCLUDED_

// ISO C headers.
#include <float.h>
#include <math.h>

// #include <janus/config.h>

//! @ingroup TYPES
//! @{

//! Signed 8-bit integer.
typedef signed char janus_int8_t;
//! Unsigned 8-bit integer.
typedef unsigned char janus_uint8_t;
//! Signed 16-bit integer.
typedef signed short janus_int16_t;
//! Unsigned 16-bit integer.
typedef unsigned short janus_uint16_t;
//! Signed 32-bit integer.
typedef signed int janus_int32_t;
//! Unsigned 32-bit integer.
typedef unsigned int janus_uint32_t;
//! Signed 64-bit integer.
typedef signed long long janus_int64_t;
//! Unsigned 64-bit integer.
typedef unsigned long long janus_uint64_t;

//! Return the maximum of a and b.
#define JANUS_MAX(a, b) (((a) > (b)) ? a : b)
//! Return the minimum of a and b.
#define JANUS_MIN(a, b) (((a) < (b)) ? a : b)

// Do not uncomment next line, use cmake -DJANUS_REAL_SINGLE=1
// #define JANUS_REAL_SINGLE
#if defined(JANUS_REAL_SINGLE)
//! Single precision floating point type.
typedef float janus_real_t;
//! Utility macro to append a suffix to a function call.
#define JANUS_REAL_SUFFIX(a) a##f
//! Utility macro to append a suffix to a number literal.
#define JANUS_REAL_CONST(a) a##f
//! Format to read real numbers using the scanf family of functions.
#define JANUS_REAL_FMT "%f"
//! Smallest number x such that 1.0 + x != 1.0
#define JANUS_REAL_EPSILON FLT_EPSILON
#else
//! Double precision floating point type.
typedef double janus_real_t;
//! Utility macro to append a suffix to a function call.
#define JANUS_REAL_SUFFIX(a) a
//! Utility macro to append a suffix to a number literal.
#define JANUS_REAL_CONST(a) a
//! Format to read real numbers using the scanf family of functions.
#define JANUS_REAL_FMT "%lf"
//! Smallest number x such that 1.0 + x != 1.0
#define JANUS_REAL_EPSILON DBL_EPSILON
#endif

#if defined(JANUS_FFTW_SINGLE) || defined(JANUS_REAL_SINGLE)
//! Utility macro to prepend a prefix to a FFTW function call or type.
#define JANUS_FFTW_PREFIX(a) fftwf_##a
#define janus_fftw_real_t float
#else
//! Utility macro to prepend a prefix to a FFTW function call or type.
#define JANUS_FFTW_PREFIX(a) fftw_##a
#define janus_fftw_real_t double
#endif

// #define JANUS_HIPRECISION_SINGLE
#if defined(JANUS_HIPRECISION_SINGLE)
//! Single precision floating point type.
typedef float janus_hiprecision_t;
//! Utility macro to append a suffix to a function call.
#define JANUS_HIPRECISION_SUFFIX(a) a##f
//! Utility macro to append a suffix to a number literal.
#define JANUS_HIPRECISION_CONST(a) a##f
//! Format to read hi precision numbers using the scanf family of functions.
#define JANUS_HIPRECISION_FMT "%f"
#else
//! Double precision floating point type.
typedef double janus_hiprecision_t;
//! Utility macro to append a suffix to a function call.
#define JANUS_HIPRECISION_SUFFIX(a) a
//! Utility macro to append a suffix to a number literal.
#define JANUS_HIPRECISION_CONST(a) a
//! Format to read hi precision numbers using the scanf family of functions.
#define JANUS_HIPRECISION_FMT "%lf"
#endif

// Microsoft Visual Studio lacks a few ISO C99 functions.
#if __STDC_VERSION__ < 199901L
//! Round v to the nearest integer (double).
#define round(v) floor(v + 0.5)
//! Round v to the nearest integer (float).
#define roundf(v) floor(v + 0.5f)
//! Return the maximum of a and b (double).
#define fmax(a, b) (((a) > (b)) ? a : b)
//! Return the maximum of a and b (float).
#define fmaxf(a, b) fmax(a, b)
//! Return the minimum of a and b (double).
#define fmin(a, b) (((a) > (b)) ? b : a)
//! Return the minimum of a and b (float).
#define fminf(a, b) fmin(a, b)
//! Return the logarithm base 2 (double).
#define log2(a) log(a) / log(2.0)
//! Return the logarithm base 2 (float).
#define log2f(a) logf(a) / logf(2.0f)
#endif

//! cos() wrapper that uses the configured floating point precision.
#define JANUS_COS(a) JANUS_REAL_SUFFIX(cos)(a)
//! sin() wrapper that uses the configured floating point precision.
#define JANUS_SIN(a) JANUS_REAL_SUFFIX(sin)(a)
//! exp() wrapper that uses the configured floating point precision.
#define JANUS_EXP(a) JANUS_REAL_SUFFIX(exp)(a)
//! fmod() wrapper that uses the configured floating point precision.
#define JANUS_FMOD(a, b) JANUS_REAL_SUFFIX(fmod)(a, b)
//! fmod() wrapper that uses the configured floating point precision.
#define JANUS_MODF(a, b) JANUS_REAL_SUFFIX(modf)(a, b)
//! fabs() wrapper that uses the configured floating point precision.
#define JANUS_FABS(a) JANUS_REAL_SUFFIX(fabs)(a)
//! fmax() wrapper that uses the configured floating point precision.
#define JANUS_FMAX(a, b) JANUS_REAL_SUFFIX(fmax)(a, b)
//! fmax() wrapper that uses the configured floating point precision.
#define JANUS_FMIM(a, b) JANUS_REAL_SUFFIX(fmin)(a, b)
//! round() wrapper that uses the configured floating point precision.
#define JANUS_ROUND(a) JANUS_REAL_SUFFIX(round)(a)
//! ceil() wrapper that uses the configured floating point precision.
#define JANUS_CEIL(a) JANUS_REAL_SUFFIX(ceil)(a)
//! floor() wrapper that uses the configured floating point precision.
#define JANUS_FLOOR(a) JANUS_REAL_SUFFIX(floor)(a)
//! trunc() wrapper that uses the configured floating point precision.
#define JANUS_TRUNC(a) JANUS_REAL_SUFFIX(trunc)(a)
//! pow() wrapper that uses the configured floating point precision.
#define JANUS_POW(a, b) JANUS_REAL_SUFFIX(pow)(a, b)
//! sqrt() wrapper that uses the configured floating point precision.
#define JANUS_SQRT(a) JANUS_REAL_SUFFIX(sqrt)(a)
//! log2() wrapper that uses the configured floating point precision.
#define JANUS_LOG2(a) JANUS_REAL_SUFFIX(log2)(a)
//! log() wrapper that uses the configured floating point precision.
#define JANUS_LOG(a) JANUS_REAL_SUFFIX(log)(a)

//! cos() wrapper that uses the configured floating point precision.
#define JANUS_HP_COS(a) JANUS_HIPRECISION_SUFFIX(cos)(a)
//! sin() wrapper that uses the configured floating point precision.
#define JANUS_HP_SIN(a) JANUS_HIPRECISION_SUFFIX(sin)(a)

//! @}

#endif

//*************************************************************************
// JANUS is a simple, robust, open standard signalling method for         *
// underwater communications. See <http://www.januswiki.org> for details. *
//*************************************************************************
// Example software implementations provided by STO CMRE are subject to   *
// Copyright (C) 2008-2018 STO Centre for Maritime Research and           *
// Experimentation (CMRE)                                                 *
//                                                                        *
// This is free software: you can redistribute it and/or modify it        *
// under the terms of the GNU General Public License version 3 as         *
// published by the Free Software Foundation.                             *
//                                                                        *
// This program is distributed in the hope that it will be useful, but    *
// WITHOUT ANY WARRANTY; without even the implied warranty of FITNESS     *
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for       *
// more details.                                                          *
//                                                                        *
// You should have received a copy of the GNU General Public License      *
// along with this program. If not, see <http://www.gnu.org/licenses/>.   *
//*************************************************************************
// Author: Ricardo Martins                                                *
//*************************************************************************

#ifndef JANUS_TRELLIS_H_INCLUDED_
#define JANUS_TRELLIS_H_INCLUDED_

// JANUS headers.
// #include <janus/export.h>
// #include <janus/types.h>

//! @ingroup CONV
//! @{

//! Trellis object.
typedef struct janus_trellis *janus_trellis_t;

struct janus_trellis {
  //! Number of input symbols.
  unsigned inp_sym_count;
  //! Number of output symbols.
  unsigned out_sym_count;
  //! Number of states.
  unsigned state_count;
  //! Outputs for all combinations of current state and current input.
  const janus_uint8_t *outputs;
  //! Next states for all combinations of current state and current input.
  const janus_uint8_t *next_states;
};

//! Retrieve JANUS default trellis object.
//! @return JANUS default trellis object.
JANUS_EXPORT const janus_trellis_t janus_trellis_default(void);

//! @}

#endif

//*************************************************************************
// JANUS is a simple, robust, open standard signalling method for         *
// underwater communications. See <http://www.januswiki.org> for details. *
//*************************************************************************
// Example software implementations provided by STO CMRE are subject to   *
// Copyright (C) 2008-2018 STO Centre for Maritime Research and           *
// Experimentation (CMRE)                                                 *
//                                                                        *
// This is free software: you can redistribute it and/or modify it        *
// under the terms of the GNU General Public License version 3 as         *
// published by the Free Software Foundation.                             *
//                                                                        *
// This program is distributed in the hope that it will be useful, but    *
// WITHOUT ANY WARRANTY; without even the implied warranty of FITNESS     *
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for       *
// more details.                                                          *
//                                                                        *
// You should have received a copy of the GNU General Public License      *
// along with this program. If not, see <http://www.gnu.org/licenses/>.   *
//*************************************************************************
// Author: Ricardo Martins                                                *
//*************************************************************************

#ifndef JANUS_VITERBI_H_INCLUDED_
#define JANUS_VITERBI_H_INCLUDED_

// JANUS headers.
// #include <janus/export.h>
// #include <janus/trellis.h>

//! @ingroup CONV
//! @{

//! Viterbi decoder object.
typedef struct janus_viterbi *janus_viterbi_t;

//! Create a Viterbi decoder object.
//! @param trellis Trellis structure.
//! @param sdc_bits Number of soft decision bits.
//! @param tbk_depth traceback depth.
//! @return Viterbi decoder object.
JANUS_EXPORT janus_viterbi_t janus_viterbi_new(janus_trellis_t trellis,
                                               unsigned sdc_bits,
                                               unsigned tbk_depth);

//! Free Viterbi decoder object.
//! @param vit Viterbi decoder object.
JANUS_EXPORT void janus_viterbi_free(janus_viterbi_t vit);

//! Decode sequence.
//! @param vit Viterbi decoder object.
//! @param inp input sequence.
//! @param inp_len size of input sequence.
//! @param out decoded sequence.
//! @param tbk_depth traceback depth.
JANUS_EXPORT void janus_viterbi_execute(janus_viterbi_t vit,
                                        const janus_uint8_t *inp,
                                        unsigned inp_len, janus_uint8_t *out,
                                        unsigned tbk_depth);

//! @}

#endif
//*************************************************************************
// JANUS is a simple, robust, open standard signalling method for         *
// underwater communications. See <http://www.januswiki.org> for details. *
//*************************************************************************
// Example software implementations provided by STO CMRE are subject to   *
// Copyright (C) 2008-2018 STO Centre for Maritime Research and           *
// Experimentation (CMRE)                                                 *
//                                                                        *
// This is free software: you can redistribute it and/or modify it        *
// under the terms of the GNU General Public License version 3 as         *
// published by the Free Software Foundation.                             *
//                                                                        *
// This program is distributed in the hope that it will be useful, but    *
// WITHOUT ANY WARRANTY; without even the implied warranty of FITNESS     *
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for       *
// more details.                                                          *
//                                                                        *
// You should have received a copy of the GNU General Public License      *
// along with this program. If not, see <http://www.gnu.org/licenses/>.   *
//*************************************************************************
// Author: Ricardo Martins                                                *
//*************************************************************************

// #include "janus.h"

static const janus_uint8_t c_default_outputs[] = {
    0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 1, 2, 3, 0, 1, 2, 3, 0, 3,
    0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0, 2, 1,
    0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3,
    0, 1, 2, 3, 0, 2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 0, 3, 2, 1,
    0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3,
    0, 1, 2, 2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 3, 0, 1, 2, 3, 0,
    1, 2, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1,
    2, 0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 1, 2, 3, 0, 1, 2, 3, 0,
    3, 0, 1, 2, 3, 0, 1, 2, 0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 2,
    1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2,
    3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0, 2, 1, 0,
    3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3,
    2, 1, 0, 3, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2, 0, 3, 2, 1, 0,
    3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0,
    1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0, 2, 1, 0, 3, 2, 1, 0,
    3, 0, 3, 2, 1, 0, 3, 2, 1, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2,
    0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 2, 1, 0, 3, 2, 1, 0, 3, 0,
    3, 2, 1, 0, 3, 2, 1, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0, 2, 1,
    0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3,
    0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2, 0, 3, 2, 1,
    0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3};

static const janus_uint8_t c_default_next_states[] = {
    0,   0,   1,   1,   2,   2,   3,   3,   4,   4,   5,   5,   6,   6,   7,
    7,   8,   8,   9,   9,   10,  10,  11,  11,  12,  12,  13,  13,  14,  14,
    15,  15,  16,  16,  17,  17,  18,  18,  19,  19,  20,  20,  21,  21,  22,
    22,  23,  23,  24,  24,  25,  25,  26,  26,  27,  27,  28,  28,  29,  29,
    30,  30,  31,  31,  32,  32,  33,  33,  34,  34,  35,  35,  36,  36,  37,
    37,  38,  38,  39,  39,  40,  40,  41,  41,  42,  42,  43,  43,  44,  44,
    45,  45,  46,  46,  47,  47,  48,  48,  49,  49,  50,  50,  51,  51,  52,
    52,  53,  53,  54,  54,  55,  55,  56,  56,  57,  57,  58,  58,  59,  59,
    60,  60,  61,  61,  62,  62,  63,  63,  64,  64,  65,  65,  66,  66,  67,
    67,  68,  68,  69,  69,  70,  70,  71,  71,  72,  72,  73,  73,  74,  74,
    75,  75,  76,  76,  77,  77,  78,  78,  79,  79,  80,  80,  81,  81,  82,
    82,  83,  83,  84,  84,  85,  85,  86,  86,  87,  87,  88,  88,  89,  89,
    90,  90,  91,  91,  92,  92,  93,  93,  94,  94,  95,  95,  96,  96,  97,
    97,  98,  98,  99,  99,  100, 100, 101, 101, 102, 102, 103, 103, 104, 104,
    105, 105, 106, 106, 107, 107, 108, 108, 109, 109, 110, 110, 111, 111, 112,
    112, 113, 113, 114, 114, 115, 115, 116, 116, 117, 117, 118, 118, 119, 119,
    120, 120, 121, 121, 122, 122, 123, 123, 124, 124, 125, 125, 126, 126, 127,
    127, 128, 128, 129, 129, 130, 130, 131, 131, 132, 132, 133, 133, 134, 134,
    135, 135, 136, 136, 137, 137, 138, 138, 139, 139, 140, 140, 141, 141, 142,
    142, 143, 143, 144, 144, 145, 145, 146, 146, 147, 147, 148, 148, 149, 149,
    150, 150, 151, 151, 152, 152, 153, 153, 154, 154, 155, 155, 156, 156, 157,
    157, 158, 158, 159, 159, 160, 160, 161, 161, 162, 162, 163, 163, 164, 164,
    165, 165, 166, 166, 167, 167, 168, 168, 169, 169, 170, 170, 171, 171, 172,
    172, 173, 173, 174, 174, 175, 175, 176, 176, 177, 177, 178, 178, 179, 179,
    180, 180, 181, 181, 182, 182, 183, 183, 184, 184, 185, 185, 186, 186, 187,
    187, 188, 188, 189, 189, 190, 190, 191, 191, 192, 192, 193, 193, 194, 194,
    195, 195, 196, 196, 197, 197, 198, 198, 199, 199, 200, 200, 201, 201, 202,
    202, 203, 203, 204, 204, 205, 205, 206, 206, 207, 207, 208, 208, 209, 209,
    210, 210, 211, 211, 212, 212, 213, 213, 214, 214, 215, 215, 216, 216, 217,
    217, 218, 218, 219, 219, 220, 220, 221, 221, 222, 222, 223, 223, 224, 224,
    225, 225, 226, 226, 227, 227, 228, 228, 229, 229, 230, 230, 231, 231, 232,
    232, 233, 233, 234, 234, 235, 235, 236, 236, 237, 237, 238, 238, 239, 239,
    240, 240, 241, 241, 242, 242, 243, 243, 244, 244, 245, 245, 246, 246, 247,
    247, 248, 248, 249, 249, 250, 250, 251, 251, 252, 252, 253, 253, 254, 254,
    255, 255};

static const struct janus_trellis c_default_trellis = {
    2, 4, 256, c_default_outputs, c_default_next_states};

const janus_trellis_t janus_trellis_default(void) {
  return (const janus_trellis_t)&c_default_trellis;
}

//*************************************************************************
// JANUS is a simple, robust, open standard signalling method for         *
// underwater communications. See <http://www.januswiki.org> for details. *
//*************************************************************************
// Example software implementations provided by STO CMRE are subject to   *
// Copyright (C) 2008-2018 STO Centre for Maritime Research and           *
// Experimentation (CMRE)                                                 *
//                                                                        *
// This is free software: you can redistribute it and/or modify it        *
// under the terms of the GNU General Public License version 3 as         *
// published by the Free Software Foundation.                             *
//                                                                        *
// This program is distributed in the hope that it will be useful, but    *
// WITHOUT ANY WARRANTY; without even the implied warranty of FITNESS     *
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for       *
// more details.                                                          *
//                                                                        *
// You should have received a copy of the GNU General Public License      *
// along with this program. If not, see <http://www.gnu.org/licenses/>.   *
//*************************************************************************
// Author: Ricardo Martins                                                *
//*************************************************************************

// ISO C headers.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

//*************************************************************************
// JANUS is a simple, robust, open standard signalling method for         *
// underwater communications. See <http://www.januswiki.org> for details. *
//*************************************************************************
// Example software implementations provided by STO CMRE are subject to   *
// Copyright (C) 2008-2018 STO Centre for Maritime Research and           *
// Experimentation (CMRE)                                                 *
//                                                                        *
// This is free software: you can redistribute it and/or modify it        *
// under the terms of the GNU General Public License version 3 as         *
// published by the Free Software Foundation.                             *
//                                                                        *
// This program is distributed in the hope that it will be useful, but    *
// WITHOUT ANY WARRANTY; without even the implied warranty of FITNESS     *
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for       *
// more details.                                                          *
//                                                                        *
// You should have received a copy of the GNU General Public License      *
// along with this program. If not, see <http://www.gnu.org/licenses/>.   *
//*************************************************************************
// Author: Ricardo Martins                                                *
//*************************************************************************

#ifndef JANUS_UTILS_MEMORY_H_INCLUDED_
#define JANUS_UTILS_MEMORY_H_INCLUDED_

// ISO C headers.
#include <stdlib.h>

#ifdef _MSC_VER
#include <malloc.h>
#ifdef JANUS_SYS_HAS___ALLOCA
#ifndef alloca
#define alloca _alloca
#endif
#endif
#else
#ifdef JANUS_SYS_HAS___ALLOCA
#include <alloca.h>
#else
// #include <malloc.h>
#endif
#endif

#define JANUS_UTILS_MEMORY_NEW(type, elms) (type *)malloc(elms * sizeof(type))

#define JANUS_UTILS_MEMORY_NEW_ZERO(type, elms)                                \
  (type *)calloc(elms, sizeof(type))

#define JANUS_UTILS_MEMORY_REALLOC(ptr, type, elms)                            \
  (type *)realloc(ptr, elms * sizeof(type))

#define JANUS_UTILS_MEMORY_FREE(ptr)                                           \
  if (ptr != 0) {                                                              \
    free(ptr);                                                                 \
    ptr = 0;                                                                   \
  }

#if defined(JANUS_SYS_HAS___ALLOCA)

#define JANUS_UTILS_MEMORY_ALLOCA(type, elms)                                  \
  (type *)alloca(elms * sizeof(type))

#define JANUS_UTILS_MEMORY_ALLOCA_FREE(ptr)                                    \
  {                                                                            \
  }

#else

#define JANUS_UTILS_MEMORY_ALLOCA(type, elms)                                  \
  (type *)malloc(elms * sizeof(type))

#define JANUS_UTILS_MEMORY_ALLOCA_FREE(ptr) free(ptr)

#endif

#endif

// JANUS headers.
// #include <janus/utils/memory.h>
// #include <janus/viterbi.h>

#define C_MAX_METRIC 999999U

struct janus_viterbi {
  // Trellis structure.
  janus_trellis_t trellis;
  // Soft decision bits.
  unsigned sdc_bits;
  // Branch metric.
  unsigned *met_brc;
  // Size of branch metric array.
  unsigned met_brc_size;
  // Temporary metric.
  unsigned *met_tmp;
  // Size of temporary metric array.
  unsigned met_tmp_size;
  // State metric.
  unsigned *met_sta;
  // Size of state metric array.
  unsigned met_sta_size;
  // Traceback depth.
  unsigned tbk_depth;
  // Traceback input.
  unsigned *tbk_inp;
  // Size of traceback input array.
  unsigned tbk_inp_size;
  // Traceback state.
  unsigned *tbk_sta;
  // Size of traceback state array.
  unsigned tbk_sta_size;
  // Number of bits per output symbol.
  unsigned k;
  // Number of bits per input symbol.
  unsigned n;
};

void reset_traceback(janus_viterbi_t vit, unsigned tbk_depth) {
  if (vit->tbk_depth == tbk_depth) {
    memset(vit->tbk_inp, 0, vit->tbk_inp_size * sizeof(unsigned));
    memset(vit->tbk_sta, 0, vit->tbk_sta_size * sizeof(unsigned));
    return;
  }

  if (vit->tbk_inp)
    free(vit->tbk_inp);

  if (vit->tbk_sta)
    free(vit->tbk_sta);

  vit->tbk_depth = tbk_depth;
  vit->tbk_inp_size = vit->trellis->state_count * (tbk_depth + 1);
  vit->tbk_inp = JANUS_UTILS_MEMORY_NEW_ZERO(unsigned, vit->tbk_inp_size);
  vit->tbk_sta_size = vit->trellis->state_count * (tbk_depth + 1);
  vit->tbk_sta = JANUS_UTILS_MEMORY_NEW_ZERO(unsigned, vit->tbk_sta_size);
}

void reset_metric(janus_viterbi_t vit) {
  memset(vit->met_sta, 0, vit->met_sta_size * sizeof(unsigned));
}

void branch_metric(janus_viterbi_t vit, const janus_uint8_t *inp,
                   unsigned idx) {
  int inp_off = idx * vit->n;
  int tmp = 0;

  int i;
  unsigned j = 0;
  for (i = 0; i < (1 << vit->n); ++i) {
    tmp = i;
    vit->met_brc[i] = 0;

    for (j = 0; j < vit->n; ++j) {
      if (tmp & 1)
        vit->met_brc[i] +=
            ((1 << vit->sdc_bits) - 1) - inp[inp_off + vit->n - j - 1];
      else
        vit->met_brc[i] += inp[inp_off + vit->n - j - 1];

      tmp >>= 1;
    }
  }
}

int update_state_metric(janus_viterbi_t vit, unsigned *tbk_idx) {
  // Next state.
  int sta_nxt = 0;
  // Current output.
  int out_cur = 0;
  unsigned renorm = C_MAX_METRIC;
  int sta_min = 0;

  unsigned i = 0;
  for (i = 0; i < vit->trellis->state_count; ++i)
    vit->met_tmp[i] = C_MAX_METRIC;

  for (i = 0; i < vit->trellis->state_count; ++i) {
    unsigned j = 0;
    for (j = 0; j < (1U << vit->k); ++j) {
      sta_nxt = vit->trellis->next_states[j * vit->trellis->state_count + i];
      out_cur = vit->trellis->outputs[j * vit->trellis->state_count + i];

      if (vit->met_sta[i] + vit->met_brc[out_cur] < vit->met_tmp[sta_nxt]) {
        vit->met_tmp[sta_nxt] = vit->met_sta[i] + vit->met_brc[out_cur];
        vit->tbk_sta[sta_nxt + (*tbk_idx * vit->trellis->state_count)] = i;
        vit->tbk_inp[sta_nxt + (*tbk_idx * vit->trellis->state_count)] = j;

        if (vit->met_tmp[sta_nxt] < renorm)
          renorm = vit->met_tmp[sta_nxt];
      }
    }
  }

  for (i = 0; i < vit->trellis->state_count; ++i) {
    vit->met_sta[i] = vit->met_tmp[i] - renorm;
    if (vit->met_sta[i] == 0)
      sta_min = i;
  }

  return sta_min;
}

int traceback(janus_viterbi_t vit, janus_uint8_t *out, unsigned blk_size,
              unsigned blk_idx, unsigned sta_min, unsigned *tbk_idx) {
  int tbwork = *tbk_idx;
  int input = 0;
  int tbk_last = tbwork;

  janus_uint8_t *blk_out = NULL;

  if (blk_idx >= vit->tbk_depth) {
    unsigned i = 0;

    for (i = 0; i < vit->tbk_depth + 1; ++i) {
      input =
          (int)(vit->tbk_inp[sta_min + (tbwork * vit->trellis->state_count)]);
      sta_min =
          (int)(vit->tbk_sta[sta_min + (tbwork * vit->trellis->state_count)]);
      tbwork = (tbwork > 0) ? tbwork - 1 : vit->tbk_depth;
    }

    blk_out = out + (((blk_idx - vit->tbk_depth) % blk_size) * vit->k);
    for (i = 0; i < vit->k; ++i) {
      blk_out[vit->k - 1 - i] = input & 1;
      input >>= 1;
    }
  }

  *tbk_idx = (*tbk_idx < vit->tbk_depth) ? *tbk_idx + 1 : 0;

  return tbk_last;
}

janus_viterbi_t janus_viterbi_new(janus_trellis_t trellis, unsigned sdc_bits,
                                  unsigned tbk_depth) {
  janus_viterbi_t vit =
      (janus_viterbi_t)calloc(1, sizeof(struct janus_viterbi));

  vit->trellis = trellis;
  vit->k = (unsigned)(JANUS_LOG2((janus_real_t)trellis->inp_sym_count));
  vit->n = (unsigned)(JANUS_LOG2((janus_real_t)trellis->out_sym_count));

  vit->sdc_bits = sdc_bits;

  vit->met_brc_size = 1 << vit->n;
  vit->met_brc = JANUS_UTILS_MEMORY_NEW_ZERO(unsigned, vit->met_brc_size);
  vit->met_tmp_size = trellis->state_count;
  vit->met_tmp = JANUS_UTILS_MEMORY_NEW_ZERO(unsigned, vit->met_tmp_size);
  vit->met_sta_size = trellis->state_count;
  vit->met_sta = JANUS_UTILS_MEMORY_NEW_ZERO(unsigned, vit->met_sta_size);

  vit->tbk_depth = 0;
  vit->tbk_inp = 0;
  vit->tbk_sta = 0;

  reset_traceback(vit, tbk_depth);
  reset_metric(vit);

  return vit;
}

void janus_viterbi_free(janus_viterbi_t vit) {
  free(vit->met_brc);
  free(vit->met_tmp);
  free(vit->met_sta);
  free(vit->tbk_inp);
  free(vit->tbk_sta);
  free(vit);
}

void janus_viterbi_execute(janus_viterbi_t vit, const janus_uint8_t *inp,
                           unsigned inp_len, janus_uint8_t *out,
                           unsigned tbk_depth) {
  printf("exec begin\n");
  // Block size.
  unsigned blk_size = inp_len / vit->n;
  // Traceback index.
  unsigned tbk_idx = 0;
  // Minimum state.
  int sta_min = 0;

  int tbk_last = 0;
  unsigned i = 0;
  int out_offs = 0;
  int input = 0;

  //   printf("RESET\n");
  reset_traceback(vit, tbk_depth);
  reset_metric(vit);

  //   printf("Main Segment begin\n");
  for (i = 0; i < blk_size; ++i) {
    branch_metric(vit, inp, i);
    sta_min = update_state_metric(vit, &tbk_idx);
    tbk_last = traceback(vit, out, blk_size, i, sta_min, &tbk_idx);
    // printf("iter %d %d %d %d\n", i, sta_min, tbk_last, tbk_idx);
  }

  for (i = 0; i < vit->tbk_inp_size; i++) {
    // printf("%d ", vit->tbk_inp[i]);
  }
  //   printf("\n");

  //   printf("Second Segment begin\n");
  for (i = 0; i < vit->tbk_depth; ++i) {
    // printf("iter %d\n", i);
    unsigned j = 0;

    out_offs = (blk_size - i - 1) * vit->k;

    input =
        (int)(vit->tbk_inp[sta_min + (tbk_last * vit->trellis->state_count)]);

    // printf("zter %d %d %d \n", i, out_offs + vit->k - 1, input);
    for (j = 0; j < vit->k; ++j) {
      out[out_offs + vit->k - 1 - j] = input & 1;
      input >>= 1;
    }

    // printf("dter %d\n", i);
    sta_min =
        (int)(vit->tbk_sta[sta_min + (tbk_last * vit->trellis->state_count)]);
    tbk_last = (tbk_last > 0) ? tbk_last - 1 : vit->tbk_depth;
  }
}

void janus_convolve(const janus_trellis_t trellis, const janus_uint8_t *inp,
                    unsigned inp_len, janus_uint8_t *out) {
  unsigned state = 0;
  unsigned idx = 0;
  unsigned n = 0;

  //   printf("JCONV %d\n\n", inp_len);

  for (n = 0; n < inp_len; ++n) {

    idx = state + inp[n] * trellis->state_count;
    // printf("JCONV %d %d %d %d\n\n", n, idx, state, 0);
    out[n << 1] = (trellis->outputs[idx] >> 1) & 1;
    out[(n << 1) + 1] = (trellis->outputs[idx] & 1);
    state = trellis->next_states[idx];
  }
}

void jvct() {
  uint8_t in[17] = {0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0};
  uint8_t out[42] = {0};
  uint8_t decoded[26] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
  void *trellis = janus_trellis_default();
  void *vit = janus_viterbi_new(trellis, 8, 9 * 5);
  janus_convolve(trellis, in, 16, out);
  for (int i = 0; i < 42; i++) {
    printf("%d", out[i]);
    if ((i + 1) % 8 == 0) {
      printf(" ");
    } else if ((i + 1) % 4 == 0) {
      printf("-");
    }
    out[i] = out[i] * 255;
  }
  printf("\n");
  janus_viterbi_execute(vit, out, 40, decoded, 20);
  //   janus_viterbi_execute(vit, out + 8, 32, decoded, 16);

  for (int i = 0; i < 26; i++) {
    // printf("%d ", decoded[i]);
    printf("%d", decoded[i]);
  }
  printf("\n");
  for (int i = 0; i < 16; i++) {
    // printf("%d ", decoded[i]);
    printf("%d", in[i]);
  }
  printf("\n");
}
// 0000-0000 0000-1110 1111-0101 1001-0001 1000 0000 00
// 0000-0000 0000 1110 1111 0101 1001 0001 0000 0000 00

static void __reset_traceback(janus_viterbi_t vit, unsigned tbk_depth) {
  if (vit->tbk_depth == tbk_depth) {
    memset(vit->tbk_inp, 0, vit->tbk_inp_size * sizeof(unsigned));
    memset(vit->tbk_sta, 0, vit->tbk_sta_size * sizeof(unsigned));
    return;
  }

  if (vit->tbk_inp)
    free(vit->tbk_inp);

  if (vit->tbk_sta)
    free(vit->tbk_sta);

  vit->tbk_depth = tbk_depth;
  vit->tbk_inp_size = vit->trellis->state_count * (tbk_depth + 1);
  vit->tbk_inp = JANUS_UTILS_MEMORY_NEW_ZERO(unsigned, vit->tbk_inp_size);
  vit->tbk_sta_size = vit->trellis->state_count * (tbk_depth + 1);
  vit->tbk_sta = JANUS_UTILS_MEMORY_NEW_ZERO(unsigned, vit->tbk_sta_size);
}

static void __reset_metric(janus_viterbi_t vit) {
  memset(vit->met_sta, 0, vit->met_sta_size * sizeof(unsigned));
}

static void __branch_metric(janus_viterbi_t vit, const janus_uint8_t *inp,
                            unsigned idx) {
  int inp_off = idx * vit->n;
  int tmp = 0;

  int i;
  unsigned j = 0;
  for (i = 0; i < (1 << vit->n); ++i) {
    tmp = i;
    vit->met_brc[i] = 0;

    for (j = 0; j < vit->n; ++j) {
      if (tmp & 1)
        vit->met_brc[i] +=
            ((1 << vit->sdc_bits) - 1) - inp[inp_off + vit->n - j - 1];
      else
        vit->met_brc[i] += inp[inp_off + vit->n - j - 1];

      tmp >>= 1;
    }
  }
}

static int __update_state_metric(janus_viterbi_t vit, unsigned *tbk_idx) {
  // Next state.
  int sta_nxt = 0;
  // Current output.
  int out_cur = 0;
  unsigned renorm = C_MAX_METRIC;
  int sta_min = 0;

  unsigned i = 0;
  for (i = 0; i < vit->trellis->state_count; ++i)
    vit->met_tmp[i] = C_MAX_METRIC;

  for (i = 0; i < vit->trellis->state_count; ++i) {
    unsigned j = 0;
    for (j = 0; j < (1U << vit->k); ++j) {
      sta_nxt = vit->trellis->next_states[j * vit->trellis->state_count + i];
      out_cur = vit->trellis->outputs[j * vit->trellis->state_count + i];

      if (vit->met_sta[i] + vit->met_brc[out_cur] < vit->met_tmp[sta_nxt]) {
        vit->met_tmp[sta_nxt] = vit->met_sta[i] + vit->met_brc[out_cur];
        vit->tbk_sta[sta_nxt + (*tbk_idx * vit->trellis->state_count)] = i;
        vit->tbk_inp[sta_nxt + (*tbk_idx * vit->trellis->state_count)] = j;

        if (vit->met_tmp[sta_nxt] < renorm)
          renorm = vit->met_tmp[sta_nxt];
      }
    }
  }

  for (i = 0; i < vit->trellis->state_count; ++i) {
    vit->met_sta[i] = vit->met_tmp[i] - renorm;
    if (vit->met_sta[i] == 0)
      sta_min = i;
  }

  return sta_min;
}

static int __traceback(janus_viterbi_t vit, janus_uint8_t *out,
                       unsigned blk_size, unsigned blk_idx, unsigned sta_min,
                       unsigned *tbk_idx) {
  int tbwork = *tbk_idx;
  int input = 0;
  int tbk_last = tbwork;

  janus_uint8_t *blk_out = NULL;

  if (blk_idx >= vit->tbk_depth) {
    unsigned i = 0;

    for (i = 0; i < vit->tbk_depth + 1; ++i) {
      input =
          (int)(vit->tbk_inp[sta_min + (tbwork * vit->trellis->state_count)]);
      sta_min =
          (int)(vit->tbk_sta[sta_min + (tbwork * vit->trellis->state_count)]);
      tbwork = (tbwork > 0) ? tbwork - 1 : vit->tbk_depth;
    }

    blk_out = out + (((blk_idx - vit->tbk_depth) % blk_size) * vit->k);
    for (i = 0; i < vit->k; ++i) {
      blk_out[vit->k - 1 - i] = input & 1;
      input >>= 1;
    }
  }

  *tbk_idx = (*tbk_idx < vit->tbk_depth) ? *tbk_idx + 1 : 0;

  return tbk_last;
}