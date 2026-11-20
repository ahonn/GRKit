#include <assert.h>
#include <stdint.h>
#include <string.h>
typedef uint32_t u32;
typedef uint8_t u8;
#include "abi.h"
#define CONTROL_BARRIER() __atomic_thread_fence(__ATOMIC_SEQ_CST)
static void cache_check(volatile void *address,u32 length,int clean);
#define CONTROL_INVALIDATE(address,length) cache_check(address,length,0)
#define CONTROL_CLEAN(address,length) cache_check(address,length,1)
#include "control_mailbox.h"

static struct control_mailbox box __attribute__((aligned(64)));
static struct control_request candidate;
static const u32 epoch[4]={1,2,3,4};

/* Each side cleans or invalidates only whole lines that it owns. */
static void cache_check(volatile void *address,u32 length,int clean) {
    uintptr_t offset=(uintptr_t)address-(uintptr_t)&box;
    assert(((uintptr_t)address&63)==0 && (length&63)==0);
    if(clean) assert(((offset==0 || offset==128) && length==64) ||
                     (offset==GRKIT_CONTROL_RESPONSE && length==CONTROL_PAYLOAD_BYTES));
    else assert((offset==64 && length==64) || (offset==GRKIT_CONTROL_PAYLOAD && length==CONTROL_PAYLOAD_BYTES));
}
/* A command of the includer: answer command 7 with the payload reversed. */
static u32 handled;
static u32 reverse(void *context,struct control_request *request,u32 *length) {
    assert(context==&handled);
    ++handled;
    if(request->command!=7) return CONTROL_UNSUPPORTED;
    for(u32 i=0;i<request->length/2;++i) {
        u8 byte=request->payload[i];
        request->payload[i]=request->payload[request->length-1-i];
        request->payload[request->length-1-i]=byte;
    }
    *length=request->length;
    return 0;
}

int main(void) {
    u8 content[CONTROL_PAYLOAD_BYTES];
    const u32 other[4]={1,2,3,5};
    u32 sequence;
    memset(content,0x71,sizeof(content));
    assert(sizeof(box)==GRKIT_CONTROL_SIZE && !(CONTROL_PAYLOAD_BYTES&63));
    control_init(&box,epoch);
    assert(control_acquire(&box,epoch,&candidate)==CONTROL_EMPTY);
    assert(!control_submit(&box,other,1,content,4));
    assert(!control_submit(&box,epoch,1,content,CONTROL_PAYLOAD_BYTES+1));
    assert(!control_submit(&box,epoch,1,0,1));
    assert(box.request==0);

    sequence=control_submit(&box,epoch,1,content,sizeof(content));
    assert(sequence==1);
    assert(!control_submit(&box,epoch,1,content,1));
    assert(!control_release(&box,sequence));
    assert(control_acquire(&box,epoch,&candidate)==CONTROL_ACCEPTED);
    assert(candidate.sequence==1);
    assert(candidate.length==sizeof(content) && !memcmp(candidate.payload,content,sizeof(content)));
    /* A poll must not execute the same request twice. */
    assert(control_acquire(&box,epoch,&candidate)==CONTROL_EMPTY);
    assert(!control_complete(&box,sequence+1,0,0,0));
    assert(!control_complete(&box,sequence,0,content,CONTROL_PAYLOAD_BYTES+1));
    assert(control_complete(&box,sequence,0,content,7));
    assert(box.response_length==7 && box.response_checksum==control_checksum(content,7));
    assert(!memcmp((const void *)box.response,content,7));
    assert(box.result==0 && box.ready==1);
    assert(!control_complete(&box,sequence,1,0,0));
    assert(!control_submit(&box,epoch,1,content,1));
    assert(control_release(&box,sequence));

    /* Torn or corrupt content is rejected before a command sees it. */
    assert(control_submit(&box,epoch,1,content,2)==2);
    box.payload[0]^=1;
    assert(control_acquire(&box,epoch,&candidate)==CONTROL_BAD_DIGEST);
    assert(control_complete(&box,2,CONTROL_BAD_DIGEST,0,0));
    assert(control_release(&box,2));
    assert(control_submit(&box,epoch,1,content,2)==3);
    box.length=CONTROL_PAYLOAD_BYTES+1;
    assert(control_acquire(&box,epoch,&candidate)==CONTROL_BAD_LENGTH);
    assert(control_complete(&box,3,CONTROL_BAD_LENGTH,0,0));
    assert(control_release(&box,3));
    assert(control_submit(&box,epoch,1,content,2)==4);
    box.request_epoch[0]=99;
    assert(control_acquire(&box,epoch,&candidate)==CONTROL_BAD_CONTEXT);
    assert(control_complete(&box,4,CONTROL_BAD_CONTEXT,0,0));
    assert(control_release(&box,4));
    assert(control_submit(&box,epoch,1,0,0)==5);
    assert(control_acquire(&box,epoch,&candidate)==CONTROL_ACCEPTED);
    assert(candidate.length==0);
    assert(control_complete(&box,5,0,0,0));
    assert(control_release(&box,5));

    /* The end of the sequence needs a new start of the camera, never a wrap. */
    box.request=box.ready=box.release=CONTROL_SEQUENCE_MAX;
    assert(!control_submit(&box,epoch,1,0,0));
    assert(control_checksum((const u8 *)"123456789",9)==0xcbf43926U);
    /* The poll accepts a first nonce once and rejects a later takeover. A
       restarted service continues with the same epoch. */
    {
        const u32 unattached[4]={0,0,0,0};
        const u8 nonce[16]={1,0,0,0,2,0,0,0,3,0,0,0,4,0,0,0};
        u8 changed_nonce[16];
        memcpy(changed_nonce,nonce,16); changed_nonce[0]=9;
        control_init(&box,unattached);
        assert(!control_poll_once(&box,&candidate,reverse,&handled));
        /* No command runs before an ATTACH. */
        assert(control_submit(&box,unattached,CONTROL_PING,0,0)==1);
        assert(control_poll_once(&box,&candidate,reverse,&handled));
        assert(box.result==CONTROL_BAD_CONTEXT);
        assert(control_release(&box,1));
        assert(control_submit(&box,unattached,7,content,4)==2);
        assert(control_poll_once(&box,&candidate,reverse,&handled));
        assert(box.result==CONTROL_BAD_CONTEXT && !handled);
        assert(control_release(&box,2));
        assert(control_submit(&box,unattached,CONTROL_ATTACH,nonce,16)==3);
        assert(control_poll_once(&box,&candidate,reverse,&handled));
        assert(box.result==0 && control_context(&box,epoch));
        assert(control_release(&box,3));
        assert(control_submit(&box,epoch,CONTROL_PING,0,0)==4);
        /* A restarted producer sees the request, not an empty slot. */
        assert(!control_submit(&box,epoch,CONTROL_PING,0,0));
        assert(control_poll_once(&box,&candidate,reverse,&handled));
        assert(box.result==0 && !box.response_length);
        assert(control_release(&box,4));
        assert(control_submit(&box,epoch,CONTROL_ATTACH,changed_nonce,16)==5);
        assert(control_poll_once(&box,&candidate,reverse,&handled));
        assert(box.result==CONTROL_BAD_CONTEXT && control_context(&box,epoch));
        assert(control_release(&box,5));
        /* The includer answers each other command, with its response. */
        assert(control_submit(&box,epoch,7,(const u8 *)"abc",3)==6);
        assert(control_poll_once(&box,&candidate,reverse,&handled));
        assert(box.result==0 && handled==1 && box.response_length==3 && !memcmp((const void *)box.response,"cba",3));
        assert(control_release(&box,6));
        assert(control_submit(&box,epoch,999,(const u8 *)"abc",3)==7);
        assert(control_poll_once(&box,&candidate,reverse,&handled));
        assert(box.result==CONTROL_UNSUPPORTED && handled==2 && !box.response_length);
        assert(control_release(&box,7));
        assert(control_submit(&box,epoch,7,0,0)==8);
        assert(control_poll_once(&box,&candidate,0,0));
        assert(box.result==CONTROL_UNSUPPORTED);
    }
    return 0;
}
