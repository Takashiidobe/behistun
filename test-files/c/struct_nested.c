#include <stdio.h>

struct point {
  int x;
  int y;
};

struct rect {
  struct point top_left;
  struct point bottom_right;
};

int main(void) {
  struct rect r = {{10, 20}, {100, 200}};

  printf("%d\n", r.top_left.x);
  printf("%d\n", r.top_left.y);
  printf("%d\n", r.bottom_right.x);
  printf("%d\n", r.bottom_right.y);

  int width = r.bottom_right.x - r.top_left.x;
  int height = r.bottom_right.y - r.top_left.y;

  printf("%d\n", width);
  printf("%d\n", height);
  printf("%d\n", width * height);

  return 0;
}
