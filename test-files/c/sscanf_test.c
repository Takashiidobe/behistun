#include <stdio.h>
#include <string.h>

int main(void) {
  // Test parsing integers
  int a, b, c;
  int result = sscanf("123 456 789", "%d %d %d", &a, &b, &c);
  if (result == 3 && a == 123 && b == 456 && c == 789) {
    printf("sscanf integers ok\n");
  }

  // Test parsing strings
  char str1[20], str2[20];
  result = sscanf("hello world", "%s %s", str1, str2);
  if (result == 2 && strcmp(str1, "hello") == 0 && strcmp(str2, "world") == 0) {
    printf("sscanf strings ok\n");
  }

  // Test parsing mixed
  int num;
  char word[20];
  result = sscanf("value 42", "%s %d", word, &num);
  if (result == 2 && strcmp(word, "value") == 0 && num == 42) {
    printf("sscanf mixed ok\n");
  }

  // Test parsing hex
  unsigned int hex;
  result = sscanf("0xABCD", "%x", &hex);
  if (result == 1 && hex == 0xABCD) {
    printf("sscanf hex ok\n");
  }

  return 0;
}
