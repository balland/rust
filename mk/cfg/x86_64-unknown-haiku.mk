# x86_64-unknown-haiku configuration
CROSS_PREFIX_x86_64-unknown-haiku=x86_64-unknown-haiku-
CC_x86_64-unknown-haiku=$(CC)
CXX_x86_64-unknown-haiku=$(CXX)
CPP_x86_64-unknown-haiku=$(CPP)
AR_x86_64-unknown-haiku=$(AR)
CFG_LIB_NAME_x86_64-unknown-haiku=lib$(1).so
CFG_STATIC_LIB_NAME_x86_64-unknown-haiku=lib$(1).a
CFG_LIB_GLOB_x86_64-unknown-haiku=lib$(1)-*.so
CFG_LIB_DSYM_GLOB_x86_64-unknown-haiku=lib$(1)-*.dylib.dSYM
CFG_CFLAGS_x86_64-unknown-haiku := -m64 $(CFLAGS)
CFG_GCCISH_CFLAGS_x86_64-unknown-haiku := -Wall -Werror -g -fPIC -m64 $(CFLAGS)
CFG_GCCISH_CXXFLAGS_x86_64-unknown-haiku := -fno-rtti $(CXXFLAGS)
CFG_GCCISH_LINK_FLAGS_x86_64-unknown-haiku := -shared -fPIC -ldl -pthread -lrt -g -m64
CFG_GCCISH_PRE_LIB_FLAGS_x86_64-unknown-haiku := -Wl,-whole-archive
CFG_GCCISH_POST_LIB_FLAGS_x86_64-unknown-haiku := -Wl,-no-whole-archive
CFG_DEF_SUFFIX_x86_64-unknown-haiku := .linux.def
CFG_LLC_FLAGS_x86_64-unknown-haiku :=
CFG_INSTALL_NAME_x86_64-unknown-haiku =
CFG_EXE_SUFFIX_x86_64-unknown-haiku =
CFG_WINDOWSY_x86_64-unknown-haiku :=
CFG_UNIXY_x86_64-unknown-haiku := 1
CFG_PATH_MUNGE_x86_64-unknown-haiku := true
CFG_LDPATH_x86_64-unknown-haiku :=
CFG_RUN_x86_64-unknown-haiku=$(2)
CFG_RUN_TARG_x86_64-unknown-haiku=$(call CFG_RUN_x86_64-unknown-haiku,,$(2))
CFG_GNU_TRIPLE_x86_64-unknown-haiku := x86_64-unknown-haiku