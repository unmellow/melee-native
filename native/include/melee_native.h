#pragma once
/* Native host C ABI. The decomp keeps using dolphin/pad.h, dolphin/gx.h, etc.
   This header is for host-only tools and the demo. */

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct PADStatus {
    uint16_t button;
    int8_t stickX;
    int8_t stickY;
    int8_t substickX;
    int8_t substickY;
    uint8_t triggerLeft;
    uint8_t triggerRight;
    uint8_t analogA;
    uint8_t analogB;
    int8_t err;
} PADStatus;

int PADInit(void);
unsigned int PADRead(PADStatus *status);
void PADClamp(PADStatus *status);
void PADControlMotor(int chan, unsigned int command);

void OSInit(void);
long long OSGetTime(void);
void VIWaitForRetrace(void);
void DVDInit(void);

void GXInit(void *base, unsigned int size);
void GXBegin(unsigned int prim, unsigned int fmt, unsigned short nverts);
void GXPosition3f32(float x, float y, float z);
void GXColor4u8(unsigned char r, unsigned char g, unsigned char b, unsigned char a);
void GXTexCoord2f32(float u, float v);
void GXEnd(void);

#ifdef __cplusplus
}
#endif
