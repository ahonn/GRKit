#ifndef GRKIT_MODE_DIAL_H
#define GRKIT_MODE_DIAL_H
/* The mode dial: what the user modes U1 to U3 do (GRKIT_HOST_RECIPE_DIAL,
   which the recipe file keeps). A user mode follows (GRKIT_RECIPE_DIAL_FOLLOW,
   and an ID that no recipe that is on has), turns GRKit off
   (GRKIT_RECIPE_DIAL_OFF), or applies a recipe by its ID. The Mode Dial page
   of MENU sets them (runtime/menu_dial.h). A user mode that
   does not follow is bound. The dial is part of CORE.RECIPE: its hook
   installs with the Recipes module (point mode.dial), also when the module
   loads stopped, and it acts while the Enable of that module is On. The
   switch Enable GRKit does not stop it, so a bound user mode can turn GRKit
   on.

   The base state is the GRKit settings while the dial is not on a bound user
   mode. The firmware keeps and restores the native settings of each
   position itself, so the core keeps only its own:
   - The dial enters a bound user mode from the base state: the core keeps a
     copy of the base state (the capability words, the values of the module
     settings, which hold the Enable of each module and the switch, and the
     Fn adjustments and the Toning of each effect), holds the main state file
     (GRKIT_HOST_STATE_HOLD), and applies the user mode: the switch Off, or
     the switch On and the recipe, without an undo step. A recipe that cannot
     apply (its effect or a native value is missing) makes the user mode
     follow: the core restores the base state and releases the file.
   - The settings that change in a bound user mode are temporary.
   - The dial moves to another bound user mode: the core applies it, and
     keeps the copy of the base state.
   - The dial leaves the bound user modes: the core restores each setting of
     the copy that differs, so a setting that did not change writes nothing
     (an unchanged Framing ratio does not set the native aspect again), and
     releases the file. The changes that the hold kept from the file then
     save at the next poll of the loader.
   - At a start with the dial on a bound user mode, the core applies it once
     the shooting screen shows (menu_ready): the firmware sends no dial event
     at power-on. The restored state file is the base state.

   The hook (menu_dial_moved) runs when the firmware applies a dial
   position. The firmware applies each position that the dial holds for
   about 20 ms, so the hook only starts the timer of the dial again, and the
   timer acts on the position where the dial stays. The firmware loads the
   native settings of a user mode in steps, so the timer waits until they
   are loaded (DIAL_NATIVE_SETTLED) and tries again a bounded number of
   times. The timer and the hook run in the App task.

   The family header gives DIAL_POSITION(), the code of the dial position,
   DIAL_CODE_U1, the code of U1 (U2 and U3 follow), and
   DIAL_NATIVE_SETTLED(position), whether the firmware has loaded the
   settings of that position. The state is in the services block
   (SERVICES_DIAL). The includer is runtime/menu.c, after the recipes, the
   module list, and Toning. */

/* The wait after the last move of the dial, and after a try while the
   firmware was still loading the native settings, and the most tries. */
#define DIAL_SETTLE_MS 300U
#define DIAL_RETRY_MS 100U
#define DIAL_TRIES 20U
/* What a user mode does now. */
#define DIAL_FOLLOW 0U
#define DIAL_OFF 1U
#define DIAL_RECIPE 2U

#ifndef DIAL_TIMER_START
#define DIAL_TIMER_START(ms) ui_timer_start(ms)
#define DIAL_TIMER_CANCEL(id) ui_timer_cancel(id)
#endif

static volatile u32 *dial_word(volatile u32 *h,u32 offset) { return services_word(h,SERVICES_DIAL+offset); }

/* The user mode of the dial position, 1 to 3, or 0 for another position. */
static u32 dial_user_mode(u32 position) {
    u32 u=position-DIAL_CODE_U1;
    return u<GRKIT_RECIPE_DIAL_POSITIONS ? u+1U : 0U;
}
/* Whether the dial acts: the Recipes module, the listed module that uses
   CORE.RECIPE, loaded at this start with its own Enable On. */
static u32 dial_enabled(volatile u32 *h) {
    u32 i;
    for(i=0;i<manage_count(h);++i)
        if(manage_status(h,i)==MANAGE_LOADED && MANAGE_MAP(manage_entry(h,i)+MANAGE_USES)[0]&GRKIT_CORE_RECIPE_BIT)
            return !manage_off(h,i);
    return 0;
}
/* What user mode u (1 to 3) does, and for DIAL_RECIPE the entry of its
   recipe in *entry. */
static u32 dial_assignment(volatile u32 *h,u32 u,u32 *entry) {
    u32 value=h[GRKIT_HOST_RECIPE_DIAL/4+u-1U], e, n=recipe_entries(h);
    if(value==GRKIT_RECIPE_DIAL_OFF) return DIAL_OFF;
    if(value==GRKIT_RECIPE_DIAL_FOLLOW) return DIAL_FOLLOW;
    for(e=0;e<n;++e) if(*recipe_id_at(h,e)==value) { *entry=e; return DIAL_RECIPE; }
    return DIAL_FOLLOW;
}

/* ---- The base state ------------------------------------------------------- */
/* The value of capability k as the core keeps it: the effect as its row,
   CORE.ASPECT as its selection (aspect_value hides a ratio in another native
   aspect), and the others as their header word. 0 for CORE.RECIPE. */
static u32 dial_core_value(volatile u32 *h,u32 k) {
    u32 handle=GRKIT_CORE_SETTING_BASE+k;
    if(handle==GRKIT_CORE_TONE) return tone_value(h);
    if(handle==GRKIT_CORE_ASPECT) return h[GRKIT_HOST_FRAME_RATIO/4];
    return core_setting_values[k] ? h[core_setting_words[k]/4] : 0U;
}
static void dial_snapshot(volatile u32 *h) {
    volatile u32 *t=settings_table(h);
    u32 i, n=t ? t[0] : 0U, records=h[GRKIT_HOST_COUNT/4];
    for(i=0;i<GRKIT_CORE_SETTING_COUNT;++i) dial_word(h,DIAL_BASE_CORE+4U*i)[0]=dial_core_value(h,i);
    dial_word(h,DIAL_BASE_COUNT)[0]=n;
    for(i=0;i<n;++i) dial_word(h,DIAL_BASE_SETTINGS+4U*i)[0]=t[SETTINGS_ENTRY(i)+1];
    if(records>GRKIT_TEXT_RECORDS) records=0;
    dial_word(h,DIAL_BASE_RECORDS)[0]=records;
    for(i=0;i<records;++i) {
        u32 record=recipe_record(h,i);
        dial_word(h,DIAL_BASE_FN+4U*i)[0]=recipe_fn_read(record);
        ((volatile u8 *)dial_word(h,DIAL_BASE_TONING))[i]=BYTE(record,GRKIT_RECORD_TONING);
    }
}
/* Set each setting of the base state that differs from the current one:
   the Fn adjustments and Toning first, so the live view builds once with
   them, then the module settings (the Enable of each module and the switch
   apply at once), then the capabilities. */
static void dial_restore(volatile u32 *h) {
    volatile u32 *t=settings_table(h);
    u32 i, n=dial_word(h,DIAL_BASE_COUNT)[0], records=dial_word(h,DIAL_BASE_RECORDS)[0], changed=0, value;
    if(records!=h[GRKIT_HOST_COUNT/4]) records=0;
    for(i=0;i<records;++i) {
        u32 record=recipe_record(h,i), stored=((volatile u8 *)dial_word(h,DIAL_BASE_TONING))[i];
        if(recipe_fn_write(record,dial_word(h,DIAL_BASE_FN+4U*i)[0])) changed=1;
        if(BYTE(record,GRKIT_RECORD_TONING)!=stored) {
            if(toning_record(record)) toning_apply(h,record,stored);
            else BYTE(record,GRKIT_RECORD_TONING)=(u8)stored;
            changed=1;
        }
    }
    if(changed) {
        ++h[GRKIT_HOST_PARAMETER_CHANGES/4];
        GRKIT_STATE_CHANGED(h);
    }
    changed=0;
    if(!t || n!=t[0]) n=0;
    for(i=0;i<n;++i) {
        value=dial_word(h,DIAL_BASE_SETTINGS+4U*i)[0];
        if(t[SETTINGS_ENTRY(i)+1]==value) continue;
        setting_store(h,i,value);
        changed=1;
    }
    if(changed) manage_off_changed(h);
    for(i=0;i<GRKIT_CORE_SETTING_COUNT;++i) {
        value=dial_word(h,DIAL_BASE_CORE+4U*i)[0];
        /* CORE.RECIPE has no value. */
        if(!core_setting_values[i] && GRKIT_CORE_SETTING_BASE+i!=GRKIT_CORE_TONE) continue;
        if(dial_core_value(h,i)!=value) setting_store(h,GRKIT_CORE_SETTING_BASE+i,value);
    }
}

/* ---- The state machine ---------------------------------------------------- */
/* Leave the bound user modes: the base state, and the state file saves
   again. */
static void dial_leave(volatile u32 *h) {
    if(!dial_word(h,DIAL_ACTIVE)[0]) return;
    dial_restore(h);
    dial_word(h,DIAL_ACTIVE)[0]=0;
    h[GRKIT_HOST_STATE_HOLD/4]=0;
}
/* Apply what user mode u does, from the base state or from another bound
   user mode. A user mode that follows, or whose recipe cannot apply, leaves
   the bound user modes. */
static void dial_enter(volatile u32 *h,u32 u) {
    u32 entry=0, kind=dial_assignment(h,u,&entry), w[GRKIT_RECIPE_WORDS], i;
    if(kind==DIAL_FOLLOW) { dial_leave(h); return; }
    if(!dial_word(h,DIAL_ACTIVE)[0]) {
        dial_snapshot(h);
        h[GRKIT_HOST_STATE_HOLD/4]=1;
    }
    dial_word(h,DIAL_ACTIVE)[0]=u;
    if(kind==DIAL_OFF) { manage_set_all_off(h,1); return; }
    /* The capabilities of the modules run only while the switch is On, so
       the switch goes On before the check and the apply. */
    if(manage_all_off(h)) manage_set_all_off(h,0);
    if(recipe_entry_reason(h,entry)!=RECIPE_READY) { dial_leave(h); return; }
    for(i=0;i<GRKIT_RECIPE_WORDS;++i) w[i]=recipe_words(h,entry)[i];
    recipe_set(h,w);
}
/* Act on user mode u (0 for another position) after the dial stays there. */
static void dial_settle(volatile u32 *h,u32 u) {
    dial_word(h,DIAL_AT)[0]=u;
    if(u && dial_enabled(h)) dial_enter(h,u);
    else dial_leave(h);
}
/* What user mode u (1 to 3) does changed (runtime/menu_dial.h): while the
   core acted on the dial there last, the new value applies at once. */
static void dial_assigned(volatile u32 *h,u32 u) {
    if(dial_word(h,DIAL_AT)[0]==u) dial_settle(h,u);
}
/* (Re)start the timer of the dial. */
static void dial_arm(volatile u32 *h,u32 ms) {
    DIAL_TIMER_CANCEL(dial_word(h,DIAL_TIMER)[0]);
    dial_word(h,DIAL_TIMER)[0]=DIAL_TIMER_START(ms);
}
/* The timer of the dial: act on the position where the dial stays, once the
   firmware has loaded its native settings. A position that the core acted
   on last changes nothing. 1 when `id` is the timer of the dial. */
static u32 dial_fired(volatile u32 *h,u32 id) {
    u32 position, u;
    if(!id || id!=dial_word(h,DIAL_TIMER)[0]) return 0;
    dial_word(h,DIAL_TIMER)[0]=0;
    position=DIAL_POSITION();
    if(!DIAL_NATIVE_SETTLED(position)) {
        if(++dial_word(h,DIAL_TRY)[0]<DIAL_TRIES) dial_word(h,DIAL_TIMER)[0]=DIAL_TIMER_START(DIAL_RETRY_MS);
        return 1;
    }
    u=dial_user_mode(position);
    if(u!=dial_word(h,DIAL_AT)[0]) dial_settle(h,u);
    return 1;
}
/* The firmware applies a dial position (the hook of point mode.dial). */
A32_ENTRY void menu_dial_moved(void) {
    volatile u32 *h=menu_header();
    if(!h[GRKIT_HOST_SERVICES/4]) return;
    dial_word(h,DIAL_TRY)[0]=0;
    dial_arm(h,DIAL_SETTLE_MS);
}
/* The first shooting screen after INSTALL (runtime/services.h:
   services_ready): the dial may be on a bound user mode since power-on. */
static void dial_ready(volatile u32 *h) {
    if(!dial_enabled(h)) return;
    dial_word(h,DIAL_TRY)[0]=0;
    dial_arm(h,DIAL_RETRY_MS);
}
#endif
