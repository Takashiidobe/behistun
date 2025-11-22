#include <stdio.h>

const char *day_name(int day) {
  switch (day) {
  case 1:
    return "Monday";
  case 2:
    return "Tuesday";
  case 3:
    return "Wednesday";
  case 4:
    return "Thursday";
  case 5:
    return "Friday";
  case 6:
    return "Saturday";
  case 7:
    return "Sunday";
  default:
    return "Invalid";
  }
}

int main(void) {
  for (int i = 1; i <= 7; i++) {
    printf("%s\n", day_name(i));
  }
  printf("%s\n", day_name(99));

  return 0;
}
