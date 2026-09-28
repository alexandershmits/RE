#include <stdio.h>
#include <string.h>
int main(void){ char buf[32];
  printf("password: "); scanf("%31s", buf);
  if (strcmp(buf, "RE50_is_c00l") == 0) { puts("Correct! FLAG{lv1a_strings}"); return 0; }
  puts("Wrong"); return 1; }