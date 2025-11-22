#define _XOPEN_SOURCE
#include <stdio.h>
#include <time.h>

int main(void) {
  const char *str = "2024-02-29 23:45:59";
  struct tm tm = {0};
  char *end = strptime(str, "%Y-%m-%d %H:%M:%S", &tm);
  if (!end) {
    printf("strptime_failed\n");
    return 1;
  }
  printf("year=%d mon=%d mday=%d hour=%d min=%d sec=%d leftover=%s\n",
         tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min,
         tm.tm_sec, end);
  return 0;
}
