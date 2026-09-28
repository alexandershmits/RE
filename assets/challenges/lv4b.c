#include <stdio.h>
#include <string.h>
/* two-stage: stage1 key unlocks stage2 algorithm */
int main(void){ char s1[16], s2[16];
  printf("stage1: "); scanf("%15s", s1);
  if (strcmp(s1, "alpha")) { puts("Wrong stage1"); return 1; }
  printf("stage2: "); scanf("%15s", s2);
  unsigned h = 0;
  for (int i = 0; s2[i]; i++) h = h * 0x1F + s2[i];
  if (h == 93998218) { puts("Correct! FLAG{lv4b_2stage}"); return 0; }
  puts("Wrong stage2"); return 1; }