#ifndef GRKIT_MODE_DIAL_H
#define GRKIT_MODE_DIAL_H
/* The mode dial. It belongs to the core: its hook (point mode.dial) and the
   shooting screen install with the core whenever INSTALL lists a module
   (runtime/installer.c), whatever the modules use and whatever their Enable
   is. It acts while the switch Enable GRKit is On (dial_enabled), and it
   never changes the switch.

   - P/Av/Tv/M, and Sn on GR IV, share one set of GRKit settings, the shared
     settings, which the main state file keeps. The P/Av/Tv/M row of the
     Mode Dial page turns GRKit off there (GRKIT_HOST_DIAL_SHARED_OFF).
   - A user mode, U1 to U3, does what P/Av/Tv/M do (GRKIT_DIAL_VALUE_SAME,
     the default), turns GRKit off (GRKIT_DIAL_VALUE_OFF), or applies its
     snapshot (GRKIT_DIAL_VALUE_SNAPSHOT).
   GRKit off by the dial is a flag of this session (DIAL_SUSPENDED), not the
   switch: the modules stop as with the switch Off (runtime/manage.h:
   manage_halted), and nothing saves it. The Enable row of ADJ turns GRKit
   on again there until the dial goes to another position (switch_apply).

   The snapshot of a user mode: Save Current Settings on the Mode Dial page
   (runtime/menu_dial.h) saves the current native settings in the native
   slot of the user mode, as the native save does, gives the slot a name and
   registers it, and keeps the current GRKit settings in the set of the user
   mode (GRKIT_HOST_DIAL_SETS): the value of each capability, with the
   effect by its recipe code (recipe_effect_code) and its Fn adjustments and
   Toning; and by their state file IDs, each module setting that the state
   file saves and the Enable of each module (dial_set_holds). A set holds no
   position or options of ADJ, no Enable at Start, not the switch, and not
   the values of the Mode Dial page. The IDs find the settings when the
   modules change: a setting that no longer exists is skipped, and an
   effect that is missing selects Film Look Off. The firmware loads the
   native settings of the slot at the dial; the core applies the set.

   The core keeps what each user mode does with a fingerprint of the name
   that it gave the slot (GRKIT_HOST_DIAL_SLOTS). A snapshot holds while
   its slot is registered and has that name (dial_slot_intact). After a
   native save, rename, or clear of the slot, or a lost slot, the core
   forgets the snapshot, and a user mode that applied it does what P/Av/Tv/M
   do (dial_check). The core checks this at each start once the shooting
   screen shows, when the Mode Dial page opens, and when the dial lands on
   a user mode. It never clears or restores a slot. GR III can give two
   user modes one box: a save for one of them makes the other lose its
   snapshot by its name. The resident loader saves the sets, the values, and
   the row of P/Av/Tv/M in their own file after GRKIT_HOST_DIAL_SET_CHANGES
   changes, and INSTALL restores them (runtime/install_recipes.h). That file
   also keeps a copy of the switch, Enable at Start, and Distance Unit
   (runtime/manage.h: manage_copy), so their restore does not depend on
   which of the two files was saved last.

   The dial lands on a position (dial_land) once the firmware has loaded its
   native settings:
   - With the switch Off, the core does nothing, and leaves a snapshot.
   - A user mode with a snapshot: from the shared settings, the core keeps a
     copy of them (the capability words, the values of the module settings,
     and the Fn adjustments and the Toning of each effect), marks the
     temporary state (GRKIT_HOST_STATE_HOLD), and applies each value of the set
     that differs from the current one, through the setters. From another
     user mode with a snapshot, it restores the shared settings first.
     Changes there are temporary. The main state export (runtime/persist.h)
     reads the shared copy for these settings and live values for globals.
   - Any other position leaves a snapshot: the core restores each setting
     that a set holds and that differs from the copy, so a setting that did
     not change writes nothing (an unchanged Framing ratio does not set the
     native aspect again), and clears the temporary-state mark. The export
     remains the same shared state. Then GRKit is off there or runs.
   - A change of the switch or of the Mode Dial page acts on the position
     at once: Off leaves a snapshot, and On lands again.
   - At a start, the core lands once the shooting screen shows
     (menu_viewfinder): the firmware sends no dial event at power-on. The
     restored state file holds the shared settings.

   The hook (menu_dial_moved) runs when the firmware applies a dial
   position. The firmware applies each position that the dial holds for
   about 20 ms, so the hook only starts the timer of the dial again, and the
   timer acts on the position where the dial stays. The firmware loads the
   native settings of a user mode in steps, so the timer waits until they
   are loaded (DIAL_NATIVE_SETTLED) and tries again a bounded number of
   times. A position that the core acted on last changes nothing. The timer
   and the hook run in the App task.

   The family header gives DIAL_POSITION(), the code of the dial position,
   DIAL_CODE_U1, the code of U1 (U2 and U3 follow), and
   DIAL_NATIVE_SETTLED(position), whether the firmware has loaded the
   settings of that position; and the slots: DIAL_SLOT_OF(u), the slot of
   user mode u (1 to 3), or 0 without one; DIAL_SLOT_SAVE(slot), the native
   save of the current settings in the slot; DIAL_SLOT_NAME_SET(slot, name,
   registered), with the address of the name as ASCII; and
   DIAL_SLOT_NAME_AT(slot) and DIAL_SLOT_REGIST_AT(slot), the addresses of
   the name of the slot and of whether it is registered. The state is in
   the services block (SERVICES_DIAL). The includer is runtime/menu.c, after
   the recipes, the module list, and Toning. */

/* The wait after the last move of the dial, and after a try while the
   firmware was still loading the native settings, and the most tries. */
#define DIAL_SETTLE_MS 300U
#define DIAL_RETRY_MS 100U
#define DIAL_TRIES 20U
/* No set, or no setting that a set holds. */
#define DIAL_NO_SET 0xffffffffU
/* DIAL_AT after a change that the position has to follow while the
   firmware loads its settings: the timer lands then on any position. */
#define DIAL_AT_AGAIN 0xffffffffU
/* The most characters of the name of a slot. */
#define DIAL_NAME_CHARS 32U
_Static_assert(DIAL_NAME_CHARS<DIAL_NAME_BYTES && DIAL_NAME_CHARS<=UI_KEYBOARD_CHARS,
               "The services block and the keyboard hold the name of a slot");
_Static_assert(DIAL_TEXT_UNITS>=RECIPE_UNITS,"The services block holds the text of an automatic name");

#ifndef DIAL_TIMER_START
#define DIAL_TIMER_START(ms) ui_timer_start(ms)
#define DIAL_TIMER_CANCEL(id) ui_timer_cancel(id)
#endif
static volatile u32 *dial_word(volatile u32 *h,u32 offset) { return services_word(h,SERVICES_DIAL+offset); }

/* The user mode of the dial position, 1 to 3, or 0 for another position. */
static u32 dial_user_mode(u32 position) {
    u32 u=position-DIAL_CODE_U1;
    return u<GRKIT_DIAL_USER_MODES ? u+1U : 0U;
}
/* Whether the dial acts: the switch Enable GRKit is On. */
static u32 dial_enabled(volatile u32 *h) { return !manage_all_off(h); }
/* Whether GRKit is off at P/Av/Tv/M. */
static u32 dial_shared_off(volatile u32 *h) { return h[GRKIT_HOST_DIAL_SHARED_OFF/4]!=0; }
/* The set of user mode u (1 to 3), and the number of its pairs, or
   DIAL_NO_SET without a set or for a count that no set has. */
static volatile u32 *dial_set(volatile u32 *h,u32 u) {
    return h+GRKIT_HOST_DIAL_SETS/4+(u-1U)*GRKIT_DIAL_SET_WORDS;
}
static u32 dial_set_pairs(volatile u32 *h,u32 u) {
    u32 n=dial_set(h,u)[GRKIT_DIAL_SET_COUNT];
    return n && n<=GRKIT_DIAL_SET_PAIRS+1U ? n-1U : DIAL_NO_SET;
}
static u32 dial_has_set(volatile u32 *h,u32 u) { return dial_set_pairs(h,u)!=DIAL_NO_SET; }
/* What user mode u (1 to 3) does (GRKIT_DIAL_SLOT_*). */
static volatile u32 *dial_slot_record(volatile u32 *h,u32 u) {
    return h+GRKIT_HOST_DIAL_SLOTS/4+(u-1U)*GRKIT_DIAL_SLOT_WORDS;
}
/* The value of user mode u (1 to 3): a snapshot without a set, and a value
   that no build gives, do what P/Av/Tv/M do. */
static u32 dial_value(volatile u32 *h,u32 u) {
    u32 value=dial_slot_record(h,u)[GRKIT_DIAL_SLOT_VALUE];
    if(value==GRKIT_DIAL_VALUE_OFF) return value;
    return value==GRKIT_DIAL_VALUE_SNAPSHOT && dial_has_set(h,u) ? value : GRKIT_DIAL_VALUE_SAME;
}

/* ---- The shared settings -------------------------------------------------- */
/* The value of capability k as the core keeps it: the effect as its row,
   CORE.ASPECT as its selection (aspect_value hides a ratio in another native
   aspect), and the others as their header word. 0 for CORE.RECIPE. */
static u32 dial_core_value(volatile u32 *h,u32 k) {
    u32 handle=GRKIT_CORE_SETTING_BASE+k;
    if(handle==GRKIT_CORE_TONE) return tone_value(h);
    if(handle==GRKIT_CORE_ASPECT) return h[GRKIT_HOST_FRAME_RATIO/4];
    return core_setting_values[k] ? h[core_setting_words[k]/4] : 0U;
}
/* Whether the copy and the sets hold capability k: each one with a value,
   but not CORE.RECIPE, and not CORE.ADJ and CORE.ADJ_MODULE, which say where
   the next ADJ opens and follow ADJ, not the dial. */
static u32 dial_core_holds(u32 k) {
    u32 handle=GRKIT_CORE_SETTING_BASE+k;
    if(handle==GRKIT_CORE_TONE) return 1U;
    return core_setting_values[k] && handle!=GRKIT_CORE_ADJ && handle!=GRKIT_CORE_ADJ_MODULE;
}
/* Whether a set holds setting k of the settings table `t`, as the copy of
   the shared settings does: a module setting that the state file saves (it
   has an ID), or the Enable of a listed module. Not the switch, the position
   or the options of a module in ADJ, the choices of the effects, Enable at
   Start, Distance Unit, or the values that the table keeps for modules that
   did not load (runtime/manage.h, runtime/install_plan.h). */
static u32 dial_set_holds(volatile u32 *h,volatile u32 *t,u32 k) {
    u32 module, n;
    if(!t[SETTINGS_ENTRY(k)] || !h[GRKIT_HOST_MANAGE/4]) return 0;
    if(k<manage_word(h,MANAGE_SETTINGS)) return 1;
    for(module=0, n=manage_count(h);module<n;++module)
        if(k==manage_off_handle(h,module)) return 1;
    return 0;
}
static void dial_snapshot(volatile u32 *h) {
    volatile u32 *t=settings_table(h);
    u32 i, n=t ? t[0] : 0U, records=h[GRKIT_HOST_COUNT/4];
    for(i=0;i<GRKIT_CORE_SETTING_COUNT;++i) dial_word(h,DIAL_BASE_CORE+4U*i)[0]=dial_core_holds(i) ? dial_core_value(h,i) : 0U;
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
/* Set each setting of the copy that a set holds and that differs from the
   current one: the Fn adjustments and Toning first, so the live view builds
   once with them, then the module settings (the Enable of each module
   applies at once), then the capabilities. */
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
        if(t[SETTINGS_ENTRY(i)+1]==value || !dial_set_holds(h,t,i)) continue;
        setting_store(h,i,value);
        changed=1;
    }
    if(changed) manage_off_changed(h);
    for(i=0;i<GRKIT_CORE_SETTING_COUNT;++i) {
        if(!dial_core_holds(i)) continue;
        value=dial_word(h,DIAL_BASE_CORE+4U*i)[0];
        if(dial_core_value(h,i)!=value) setting_store(h,GRKIT_CORE_SETTING_BASE+i,value);
    }
}

/* ---- The sets ------------------------------------------------------------- */
/* The handle of the setting with state file ID `id` that a set holds, or
   DIAL_NO_SET. */
static u32 dial_set_handle(volatile u32 *h,volatile u32 *t,u32 id) {
    u32 k;
    for(k=0;k<t[0];++k)
        if(t[SETTINGS_ENTRY(k)]==id && dial_set_holds(h,t,k)) return k;
    return DIAL_NO_SET;
}
/* Keep the current GRKit settings in the set of user mode u (1 to 3), which
   replaces the set that it had. The loader saves the sets after the count
   of changes. */
static void dial_save(volatile u32 *h,u32 u) {
    volatile u32 *s=dial_set(h,u), *t=settings_table(h);
    u32 i, n=0, record=h[GRKIT_HOST_ACTIVE/4], effect=0, fn=0, toning=0;
    for(i=0;i<GRKIT_CORE_SETTING_COUNT;++i) s[GRKIT_DIAL_SET_CORE+i]=dial_core_holds(i) ? dial_core_value(h,i) : 0U;
    /* The effect by its code, which finds it when the effects change. */
    if(record_index(h,record)<GRKIT_TEXT_RECORDS) {
        effect=GRKIT_DIAL_SET_EFFECT|recipe_effect_code(record);
        fn=recipe_fn_read(record);
        toning=BYTE(record,GRKIT_RECORD_TONING);
    }
    s[GRKIT_DIAL_SET_CORE+CORE_SETTING(GRKIT_CORE_TONE)]=effect;
    s[GRKIT_DIAL_SET_FN]=fn;
    s[GRKIT_DIAL_SET_TONING]=toning;
    for(i=0;t && i<t[0] && n<GRKIT_DIAL_SET_PAIRS;++i) {
        if(!dial_set_holds(h,t,i)) continue;
        s[GRKIT_DIAL_SET_PAIR+2U*n]=t[SETTINGS_ENTRY(i)];
        s[GRKIT_DIAL_SET_PAIR+2U*n+1U]=t[SETTINGS_ENTRY(i)+1];
        ++n;
    }
    /* The file keeps no pair of the set before. */
    for(i=2U*n;i<2U*GRKIT_DIAL_SET_PAIRS;++i) s[GRKIT_DIAL_SET_PAIR+i]=0;
    s[GRKIT_DIAL_SET_COUNT]=n+1U;
    ++h[GRKIT_HOST_DIAL_SET_CHANGES/4];
}
/* Apply the set of user mode u (1 to 3), which has one: each value that
   differs from the current one, through the setters, in the order of
   dial_restore. A pair whose setting the table does not hold is skipped, as
   is a value that the setting does not offer. A missing effect of the set
   selects Film Look Off. */
static void dial_set_apply(volatile u32 *h,u32 u) {
    volatile u32 *s=dial_set(h,u), *t=settings_table(h);
    u32 n=dial_set_pairs(h,u), i, k, value, record, changed=0, index=GRKIT_TEXT_RECORDS;
    u32 effect=s[GRKIT_DIAL_SET_CORE+CORE_SETTING(GRKIT_CORE_TONE)];
    if(effect) index=recipe_effect(h,effect&GRKIT_RECIPE_CODE_MASK);
    if(index<GRKIT_TEXT_RECORDS) {
        record=recipe_record(h,index);
        changed=recipe_fn_write(record,s[GRKIT_DIAL_SET_FN]);
        value=s[GRKIT_DIAL_SET_TONING]&0xffU;
        if(BYTE(record,GRKIT_RECORD_TONING)!=value) {
            if(toning_record(record)) toning_apply(h,record,value);
            else BYTE(record,GRKIT_RECORD_TONING)=(u8)value;
            changed=1;
        }
        if(changed) {
            ++h[GRKIT_HOST_PARAMETER_CHANGES/4];
            GRKIT_STATE_CHANGED(h);
        }
    }
    changed=0;
    for(i=0;t && i<n;++i) {
        k=dial_set_handle(h,t,s[GRKIT_DIAL_SET_PAIR+2U*i]);
        value=s[GRKIT_DIAL_SET_PAIR+2U*i+1U];
        if(k==DIAL_NO_SET || t[SETTINGS_ENTRY(k)+1]==value) continue;
        setting_store(h,k,value);
        changed=1;
    }
    if(changed) manage_off_changed(h);
    for(i=0;i<GRKIT_CORE_SETTING_COUNT;++i) {
        if(!dial_core_holds(i)) continue;
        if(GRKIT_CORE_SETTING_BASE+i==GRKIT_CORE_TONE) value=index<GRKIT_TEXT_RECORDS ? index+1U : 0;
        else value=s[GRKIT_DIAL_SET_CORE+i];
        if(dial_core_value(h,i)!=value) setting_store(h,GRKIT_CORE_SETTING_BASE+i,value);
    }
}

/* (Re)start the timer of the dial. */
static void dial_arm(volatile u32 *h,u32 ms) {
    DIAL_TIMER_CANCEL(dial_word(h,DIAL_TIMER)[0]);
    dial_word(h,DIAL_TIMER)[0]=DIAL_TIMER_START(ms);
}

/* ---- The native slots ----------------------------------------------------- */
#ifdef DIAL_BOX_ASSIGN
/* GR III: the box of user mode u, or 0. */
static u32 dial_box_of(u32 u) {
    u32 box=BYTE(DIAL_RECORD(),DIAL_BOX_ASSIGN+2U*(u-1U));
    return box-1U<DIAL_BOXES ? box : 0U;
}
#endif
#ifdef DIAL_NAME_RECORDS
/* GR III: the name and the registered flag of a box in each record, as the
   setters of the firmware write them: the name up to its 0, then 0. */
static void dial_names_write(u32 slot,u32 name,u32 registered) {
    u32 r, i, record, ended;
    for(r=0;r<DIAL_NAME_RECORDS;++r) {
        record=DIAL_NAME_RECORD(r);
        for(i=0, ended=0;i<DIAL_NAME_CHARS;++i) {
            if(!BYTE(name,i)) ended=1;
            BYTE(record,DIAL_BOX_NAME_FIELD(slot)+i)=ended ? 0U : BYTE(name,i);
        }
        BYTE(record,DIAL_BOX_REGIST_FIELD(slot))=(u8)registered;
    }
}
#endif
/* A fingerprint of the ASCII name at `name`, up to its 0, at most
   DIAL_NAME_CHARS, below 2^29 and not 0. */
static u32 dial_name_hash(u32 name) {
    u32 hash=0x811c9dc5U, i, c;
    for(i=0;i<DIAL_NAME_CHARS && (c=BYTE(name,i));++i) hash=(hash^c)*0x01000193U;
    return hash>>3 ? hash>>3 : 1U;
}
/* Whether user mode u (1 to 3) has a snapshot whose slot holds what the core
   saved: the slot is registered and has the name that the core gave it. */
static u32 dial_slot_intact(volatile u32 *h,u32 u) {
    u32 slot=DIAL_SLOT_OF(u);
    return dial_has_set(h,u) && slot && BYTE(DIAL_SLOT_REGIST_AT(slot),0) &&
           dial_name_hash(DIAL_SLOT_NAME_AT(slot))==dial_slot_record(h,u)[GRKIT_DIAL_SLOT_NAME];
}
/* Forget the snapshot of user mode u: its set and the name of its slot. A
   user mode that applied it does what P/Av/Tv/M do; GRKit Off stays. The
   slot stays as it is. */
static void dial_forget(volatile u32 *h,u32 u) {
    volatile u32 *s=dial_set(h,u), *record=dial_slot_record(h,u);
    u32 w;
    for(w=0;w<GRKIT_DIAL_SET_WORDS;++w) s[w]=0;
    record[GRKIT_DIAL_SLOT_NAME]=0;
    if(record[GRKIT_DIAL_SLOT_VALUE]==GRKIT_DIAL_VALUE_SNAPSHOT) record[GRKIT_DIAL_SLOT_VALUE]=GRKIT_DIAL_VALUE_SAME;
    ++h[GRKIT_HOST_DIAL_SET_CHANGES/4];
}
/* Let go of the snapshot of user mode u when its slot no longer holds what
   the core saved. The firmware has loaded the settings of the dial
   position. */
static void dial_check(volatile u32 *h,u32 u) {
    volatile u32 *record=dial_slot_record(h,u);
    if((dial_has_set(h,u) || record[GRKIT_DIAL_SLOT_VALUE]==GRKIT_DIAL_VALUE_SNAPSHOT || record[GRKIT_DIAL_SLOT_NAME]) &&
       !dial_slot_intact(h,u))
        dial_forget(h,u);
}
/* Check each user mode, while the firmware has loaded the settings of the
   dial position; else the timer of the dial does it then (dial_fired). */
static void dial_slots_check(volatile u32 *h) {
    u32 u;
    if(!DIAL_NATIVE_SETTLED(DIAL_POSITION())) {
        dial_word(h,DIAL_SYNC)[0]=1U;
        if(!dial_word(h,DIAL_TIMER)[0]) {
            dial_word(h,DIAL_TRY)[0]=0;
            dial_arm(h,DIAL_RETRY_MS);
        }
        return;
    }
    dial_word(h,DIAL_SYNC)[0]=0;
    for(u=1;u<=GRKIT_DIAL_USER_MODES;++u) dial_check(h,u);
}
/* The automatic name of a snapshot, for the keyboard: the name of the
   recipe that the current settings match, its automatic name in English, the
   name of the Film Look effect, or GRKit when none of them has a printable
   ASCII character. */
static const volatile u16 *dial_automatic_name(volatile u32 *h) {
    static const u16 grkit[6]={'G','R','K','i','t',0};
    volatile u16 *to=(volatile u16 *)dial_word(h,DIAL_TEXT);
    u32 e=recipe_matching(h,0), record=h[GRKIT_HOST_ACTIVE/4], i, printable=0;
    to[0]=0;
    if(e!=RECIPE_NONE) {
        if(recipe_name_length(h,e)) recipe_append_name(h,to,0,e);
        else recipe_append_automatic(h,to,0,e,1);
    }
    else if(record_index(h,record)<GRKIT_TEXT_RECORDS) recipe_append_effect(to,0,record);
    for(i=0;to[i] && !printable;++i) printable=to[i]>0x20U && to[i]<=0x7eU;
    if(!printable) recipe_append(to,0,grkit);
    return to;
}

/* The name of a slot in DIAL_NAME: the printable ASCII of `typed`, at most
   DIAL_NAME_CHARS, then 0 up to DIAL_NAME_BYTES. A text without a printable
   character other than a space gives the automatic name. */
static u32 dial_name(volatile u32 *h,const volatile u8 *typed) {
    const volatile u16 *text;
    u32 name=(u32)dial_word(h,DIAL_NAME), n=0, i, seen=0;
    for(i=0;typed[i] && n<DIAL_NAME_CHARS;++i)
        if(typed[i]>=0x20U && typed[i]<=0x7eU) { seen|=typed[i]!=' '; BYTE(name,n++)=typed[i]; }
    if(!seen) {
        text=dial_automatic_name(h);
        for(i=0, n=0;text[i] && n<DIAL_NAME_CHARS;++i)
            if(text[i]>=0x20U && text[i]<=0x7eU) BYTE(name,n++)=(u8)text[i];
    }
    while(n<DIAL_NAME_BYTES) BYTE(name,n++)=0;
    return name;
}
/* ---- The state machine ---------------------------------------------------- */
/* GRKit is off at this position by the dial, or not: the modules stop or
   run as with the switch (runtime/manage.h). */
static void dial_suspend(volatile u32 *h,u32 off) {
    volatile u32 *at=dial_word(h,DIAL_SUSPENDED);
    if(at[0]==off) return;
    at[0]=off;
    manage_off_changed(h);
}
/* Leave a snapshot: restore the shared settings that the export kept. */
static void dial_leave(volatile u32 *h) {
    if(!dial_word(h,DIAL_ACTIVE)[0]) return;
    dial_restore(h);
    dial_word(h,DIAL_ACTIVE)[0]=0;
    h[GRKIT_HOST_STATE_HOLD/4]=0;
}
/* Apply the snapshot of user mode u, from the shared settings or from
   another snapshot. The snapshot that applies changes nothing. */
static void dial_enter(volatile u32 *h,u32 u) {
    u32 active=dial_word(h,DIAL_ACTIVE)[0];
    if(active==u) return;
    if(!active) {
        dial_snapshot(h);
        h[GRKIT_HOST_STATE_HOLD/4]=1;
    }
    else dial_restore(h);
    dial_word(h,DIAL_ACTIVE)[0]=u;
    dial_set_apply(h,u);
}
/* Act on dial position `position`, whose native settings the firmware has
   loaded. */
static void dial_land(volatile u32 *h,u32 position) {
    u32 u=dial_user_mode(position), value=GRKIT_DIAL_VALUE_SAME;
    dial_word(h,DIAL_AT)[0]=position+1U;
    if(!dial_enabled(h)) {
        dial_leave(h);
        dial_suspend(h,0);
        return;
    }
    if(u) {
        dial_check(h,u);
        value=dial_value(h,u);
    }
    if(value==GRKIT_DIAL_VALUE_SNAPSHOT) {
        dial_suspend(h,0);
        dial_enter(h,u);
        return;
    }
    dial_leave(h);
    dial_suspend(h,value==GRKIT_DIAL_VALUE_OFF || dial_shared_off(h));
}
/* Act on the position again after a change of what it does: at once while
   the firmware has loaded its settings, else when the timer finds them.
   Nothing before the first shooting screen (dial_ready). */
static void dial_land_again(volatile u32 *h) {
    u32 position;
    if(!dial_word(h,DIAL_AT)[0]) return;
    position=DIAL_POSITION();
    if(DIAL_NATIVE_SETTLED(position)) {
        dial_land(h,position);
        return;
    }
    dial_word(h,DIAL_AT)[0]=DIAL_AT_AGAIN;
    dial_word(h,DIAL_TRY)[0]=0;
    dial_arm(h,DIAL_RETRY_MS);
}
/* The switch Enable GRKit changed (runtime/manage.h: manage_set_all_off):
   Off leaves a snapshot at once, and On lands again. */
static void dial_master_changed(volatile u32 *h) {
    if(!h[GRKIT_HOST_SERVICES/4] || !dial_word(h,DIAL_AT)[0]) return;
    if(dial_enabled(h)) {
        dial_land_again(h);
        return;
    }
    dial_leave(h);
    dial_suspend(h,0);
}
/* Save Current Settings in user mode u (1 to 3) with the name `typed`
   (runtime/menu_dial.h): the native save of the current settings in its
   slot, the name, which registers the slot, and the snapshot of the GRKit
   settings, which the user mode then applies. No current setting changes;
   on that user mode, the snapshot applies at once with nothing to set. 0,
   and nothing changes, while the firmware loads the settings of the dial
   position, or when the user mode has no slot. */
static u32 dial_snapshot_save(volatile u32 *h,u32 u,const volatile u8 *typed) {
    volatile u32 *record=dial_slot_record(h,u);
    u32 slot=DIAL_SLOT_OF(u), name;
    if(!DIAL_NATIVE_SETTLED(DIAL_POSITION()) || !slot) return 0;
    name=dial_name(h,typed);
    DIAL_SLOT_SAVE(slot);
    DIAL_SLOT_NAME_SET(slot,name,1U);
    dial_save(h,u);
    record[GRKIT_DIAL_SLOT_NAME]=dial_name_hash(name);
    record[GRKIT_DIAL_SLOT_VALUE]=GRKIT_DIAL_VALUE_SNAPSHOT;
    /* GR III: a user mode that shares the box lets go of its snapshot. */
    dial_slots_check(h);
    dial_land_again(h);
    return 1;
}
/* The timer of the dial: act on the position where the dial stays, once the
   firmware has loaded its native settings, after the check of the slots
   that waits. A position that the core acted on last changes nothing. 1
   when `id` is the timer of the dial. */
static u32 dial_fired(volatile u32 *h,u32 id) {
    u32 position;
    if(!id || id!=dial_word(h,DIAL_TIMER)[0]) return 0;
    dial_word(h,DIAL_TIMER)[0]=0;
    position=DIAL_POSITION();
    if(!DIAL_NATIVE_SETTLED(position)) {
        if(++dial_word(h,DIAL_TRY)[0]<DIAL_TRIES) dial_word(h,DIAL_TIMER)[0]=DIAL_TIMER_START(DIAL_RETRY_MS);
        return 1;
    }
    if(dial_word(h,DIAL_SYNC)[0]) dial_slots_check(h);
    if(position+1U!=dial_word(h,DIAL_AT)[0]) dial_land(h,position);
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
   services_ready): the slots wait for a check, and the dial lands where it
   is since power-on. */
static void dial_ready(volatile u32 *h) {
    dial_word(h,DIAL_SYNC)[0]=1U;
    dial_word(h,DIAL_TRY)[0]=0;
    dial_arm(h,DIAL_RETRY_MS);
}
#endif
