// Project-owned driver: executes local game bytecode without modifying its classes.
import java.lang.reflect.*;
import java.util.*;
import com.badlogic.gdx.*;
import com.badlogic.gdx.backends.headless.*;
import com.badlogic.gdx.backends.headless.mock.audio.*;
import com.badlogic.gdx.graphics.*;
import com.badlogic.gdx.graphics.g2d.*;
import com.megacrit.cardcrawl.core.*;
import com.megacrit.cardcrawl.localization.*;
import com.megacrit.cardcrawl.cards.*;
import com.megacrit.cardcrawl.characters.*;
import com.megacrit.cardcrawl.dungeons.*;
import com.megacrit.cardcrawl.helpers.*;
import com.megacrit.cardcrawl.monsters.*;
import com.megacrit.cardcrawl.monsters.exordium.*;
import com.megacrit.cardcrawl.actions.*;
import com.megacrit.cardcrawl.rooms.*;
import com.megacrit.cardcrawl.map.*;
import com.megacrit.cardcrawl.vfx.*;
public class CombatOracle {
    static sun.misc.Unsafe unsafe;
    static {
        try {
            Field f=sun.misc.Unsafe.class.getDeclaredField("theUnsafe");
            f.setAccessible(true);
            unsafe=(sun.misc.Unsafe)f.get(null);
        }
        catch(Exception e){
            throw new RuntimeException(e);
        }
    }
    static <T>T alloc(Class<T> c)throws Exception{
        return (T)unsafe.allocateInstance(c);
    }
    static Object zero(Class<?> c){
        if(c==boolean.class)return false;
        if(c==int.class)return 0;
        if(c==float.class)return 0f;
        if(c==long.class)return 0L;
        return null;
    }
    static <T>T proxy(Class<T> c){
        return (T)Proxy.newProxyInstance(c.getClassLoader(), new Class[]{c}, (o,m,a)->{
            if(m.getName().equals("getDisplayMode"))
                return new Graphics.DisplayMode(1920,1080,60,32){};
            if(m.getName().equals("getDisplayModes"))
                return new Graphics.DisplayMode[]{new Graphics.DisplayMode(1920,1080,60,32){}};
            if(m.getName().equals("getDeltaTime")) return 0.05f;
            if(m.getName().equals("getType")) return Application.ApplicationType.HeadlessDesktop;
            return zero(m.getReturnType());
        });
    }
    static void set(Object o,Class<?> c,String name,Object value)throws Exception{
        Field f=c.getDeclaredField(name);
        f.setAccessible(true);
        f.set(o,value);
    }
    // Constructor bypass is confined to controlled actors and UI bookkeeping.
    // No combat/card/action implementation is replaced or transformed.
    static void init(Object o)throws Exception{
        for(Class<?> c=o.getClass();c!=Object.class;c=c.getSuperclass())for(Field f:c.getDeclaredFields()){
            if(Modifier.isStatic(f.getModifiers()))continue;
            f.setAccessible(true);
            if(f.getType()==ArrayList.class)f.set(o,new ArrayList<>());
            if(f.getType()==Color.class)f.set(o,Color.WHITE.cpy());
            if(f.getType()==PowerTip.class)f.set(o,new PowerTip("", ""));
            if(f.getType()==Hitbox.class)f.set(o,new Hitbox(1,1));
        }
    }
    static void initialize()throws Exception{
        Gdx.input=proxy(Input.class);
        Gdx.files=new HeadlessFiles();
        Gdx.app=proxy(Application.class);
        Gdx.graphics=proxy(Graphics.class);
        Gdx.audio=new MockAudio();
        Gdx.gl=Gdx.gl20=proxy(GL20.class);
        Settings.language=Settings.GameLanguage.ENG;
        Settings.FAST_MODE=true;
        Settings.scale=1;
        Settings.WIDTH=1920;
        Settings.HEIGHT=1080;
        for(Field f:Settings.class.getDeclaredFields())if(f.getType()==Prefs.class){
            f.setAccessible(true);
            f.set(null,new Prefs(){
                public void flush(){
                }
            });
        }
        for(Field f:com.megacrit.cardcrawl.unlock.UnlockTracker.class.getDeclaredFields())if(f.getType()==Prefs.class){
            f.setAccessible(true);
            f.set(null,new Prefs(){
                public void flush(){
                }
            });
        }
        CardCrawlGame.languagePack=new LocalizedStrings();
        TipTracker.tips.put(TipTracker.SHUFFLE_TIP,true);
        com.badlogic.gdx.utils.GdxNativesLoader.load();
        Texture texture=new Texture(1,1,Pixmap.Format.RGBA8888);
        TextureAtlas atlas=new TextureAtlas(){
            public AtlasRegion findRegion(String name){
                return new AtlasRegion(texture,0,0,1,1);
            }
        };
        for(Class<?> c:new Class[]{
            AbstractCard.class,ImageMaster.class,FontHelper.class,com.megacrit.cardcrawl.powers.AbstractPower.class
        }) for(Field f:c.getDeclaredFields()){
            if(!Modifier.isStatic(f.getModifiers())||Modifier.isFinal(f.getModifiers()))continue;
            f.setAccessible(true);
            if(f.getType()==TextureAtlas.class)f.set(null,atlas);
            if(f.getType()==TextureAtlas.AtlasRegion.class)f.set(null,atlas.findRegion(""));
            if(f.getType()==Texture.class)f.set(null,texture);
            if(f.getType()==BitmapFont.class)f.set(null,new BitmapFont(new BitmapFont.BitmapFontData(),new TextureRegion(texture),false));
        }
        CardCrawlGame.sound=alloc(com.megacrit.cardcrawl.audio.SoundMaster.class);
        set(CardCrawlGame.sound,com.megacrit.cardcrawl.audio.SoundMaster.class,"map",new HashMap<>());
    }
    static Ironclad player;
    static Cultist monster;
    static AbstractGameAction current;
    static String scenario;
    static int step;
    static List<AbstractCard> logicalHand;
    static AbstractCard inUse;
    static List<String> actionLog = new ArrayList<>();
    static AbstractCard card(String spec) throws Exception {
        boolean plus=spec.endsWith("+");
        String name=plus?spec.substring(0,spec.length()-1):spec;
        if(name.equals("Strike"))name="Strike_Red";
        if(name.equals("Defend"))name="Defend_Red";
        String pkg=Set.of("Wound","Dazed","Slimed").contains(name)?"status":"red";
        AbstractCard c=(AbstractCard)Class.forName("com.megacrit.cardcrawl.cards."+pkg+"."+name).getConstructor().newInstance();
        if(plus)c.upgrade();
        return c;
    }
    static void pile(CardGroup group,String names,boolean reverse)throws Exception {
        if(!names.equals("-"))for(String name:names.split(","))group.group.add(card(name));
        if(reverse)Collections.reverse(group.group);
    }
    static void powers(AbstractCreature owner,String specs)throws Exception {
        if(specs.equals("-"))return;
        for(String spec:specs.split(",")){
            String[] parts=spec.split(":");
            String name=parts[0];
            int n=Integer.parseInt(parts[1]);
            Class<?> c=Class.forName("com.megacrit.cardcrawl.powers."+name+"Power");
            com.megacrit.cardcrawl.powers.AbstractPower power;
            if(Set.of("Weak","Vulnerable","Frail").contains(name))power=(com.megacrit.cardcrawl.powers.AbstractPower)c.getConstructor(AbstractCreature.class,int.class,boolean.class).newInstance(owner,n,false);
            else power=(com.megacrit.cardcrawl.powers.AbstractPower)c.getConstructor(AbstractCreature.class,int.class).newInstance(owner,n);
            owner.powers.add(power);
        }
    }
    static void reset(String[] row)throws Exception {
        scenario=row[1];
        step=0;
        current=null;
        inUse=null;
        logicalHand=null;
        actionLog.clear();
        AbstractDungeon.actionManager=new GameActionManager();
        init(AbstractDungeon.topPanel);
        CardCrawlGame.dungeon=alloc(com.megacrit.cardcrawl.dungeons.Exordium.class);
        AbstractDungeon.isScreenUp=false;
        AbstractDungeon.screen=AbstractDungeon.CurrentScreen.NONE;
        AbstractDungeon.handCardSelectScreen.selectedCards.clear();
        AbstractDungeon.handCardSelectScreen.wereCardsRetrieved=true;
        AbstractDungeon.gridSelectScreen.selectedCards.clear();
        long seed=Long.parseLong(row[2]);
        AbstractDungeon.aiRng=new com.megacrit.cardcrawl.random.Random(seed);
        AbstractDungeon.cardRandomRng=new com.megacrit.cardcrawl.random.Random(seed);
        AbstractDungeon.shuffleRng=new com.megacrit.cardcrawl.random.Random(seed);
        AbstractDungeon.monsterHpRng=new com.megacrit.cardcrawl.random.Random(seed);
        player=alloc(Ironclad.class);
        init(player);
        player.isPlayer=true;
        player.currentHealth=player.maxHealth=80;
        player.hand=new CardGroup(CardGroup.CardGroupType.HAND);
        player.drawPile=new CardGroup(CardGroup.CardGroupType.DRAW_PILE);
        player.discardPile=new CardGroup(CardGroup.CardGroupType.DISCARD_PILE);
        player.exhaustPile=new CardGroup(CardGroup.CardGroupType.EXHAUST_PILE);
        player.masterDeck=new CardGroup(CardGroup.CardGroupType.MASTER_DECK);
        player.limbo=new CardGroup(CardGroup.CardGroupType.UNSPECIFIED);
        player.stance=new com.megacrit.cardcrawl.stances.NeutralStance();
        player.energy=new EnergyManager(3);
        player.tint=new TintEffect();
        AbstractDungeon.player=player;
        AbstractDungeon.overlayMenu=new OverlayMenu(player);
        monster=alloc(Cultist.class);
        init(monster);
        monster.currentHealth=monster.maxHealth=200;
        monster.tint=new TintEffect();
        monster.damage.add(new DamageInfo(monster,6));
        set(monster,AbstractMonster.class,"move",new EnemyMoveInfo((byte)1,AbstractMonster.Intent.ATTACK,6,1,false));
        monster.intent=AbstractMonster.Intent.ATTACK;
        MonsterRoom room=alloc(MonsterRoom.class);
        room.souls=new SoulGroup();
        room.phase=AbstractRoom.RoomPhase.COMBAT;
        room.monsters=new MonsterGroup(monster);
        MapRoomNode node=new MapRoomNode(0,0);
        node.room=room;
        AbstractDungeon.currMapNode=node;
        pile(player.hand,row[3],false);
        pile(player.drawPile,row[4],true);
        pile(player.discardPile,row[5],false);
        powers(player,row[6]);
        powers(monster,row[7]);
        player.currentBlock=Integer.parseInt(row[8]);
        monster.currentBlock=Integer.parseInt(row[9]);
        com.megacrit.cardcrawl.ui.panels.EnergyPanel.totalCount=3;
    }
    // Drive original update() methods in queue order, pausing for real selection
    // screens. This is an isolated action scheduler, not the full game update loop.
    static void drain() throws Exception {
        int ticks=0;
        while(current!=null||!AbstractDungeon.actionManager.actions.isEmpty()){
            if(++ticks>10000)throw new IllegalStateException("queue exceeded 10000 updates: "+current);
            if(current==null){
                current=AbstractDungeon.actionManager.actions.remove(0);
                actionLog.add(current.getClass().getSimpleName());
            }
            AbstractDungeon.getCurrRoom().souls.update();
            current.update();
            if(AbstractDungeon.isScreenUp){
                if(current.isDone)throw new IllegalStateException("clock completed an action while opening a choice");
                return;
            }
            if(current.isDone)current=null;
        }
        inUse=null;
        logicalHand=null;
    }
    static String phase(){
        if(!AbstractDungeon.isScreenUp)return "play";
        String name=current.getClass().getSimpleName();
        return switch(name){
            case "ArmamentsAction"->"upgrade";
            case "ExhaustAction"->"exhaust";
            case "PutOnDeckAction"->"topdeck_hand";
            case "DiscardPileToTopOfDeckAction"->"topdeck_discard";
            default->throw new IllegalStateException("unsupported prompt "+name);
        };
    }
    static List<AbstractCard> options(){
        return phase().equals("topdeck_discard")?player.discardPile.group:player.hand.group;
    }
    static String spec(AbstractCard c){
        return c.getClass().getSimpleName().replace("_Red","")+(c.upgraded?"+":"")+":"+c.costForTurn;
    }
    static String cards(List<AbstractCard> list,boolean reverse){
        List<String> out=new ArrayList<>();
        for(AbstractCard c:list)out.add(spec(c));
        if(reverse)Collections.reverse(out);
        return out.isEmpty()?"-":String.join(",",out);
    }
    static String powers(AbstractCreature owner){
        List<String> out=new ArrayList<>();
        for(com.megacrit.cardcrawl.powers.AbstractPower p:owner.powers)out.add(p.getClass().getSimpleName().replace("Power","")+":"+p.amount);
        Collections.sort(out);
        return out.isEmpty()?"-":String.join(",",out);
    }
    static void snapshot(){
        String phase=phase();
        System.out.println("QUEUE\t"+scenario+"\t"+step+"\t"+(current==null?"-":current.getClass().getSimpleName())+"\t"+(actionLog.isEmpty()?"-":String.join(",",actionLog)));
        actionLog.clear();
        // Armaments temporarily hides ineligible cards in its selection UI. Restore
        // their logical presence only in the snapshot, without changing game state.
        List<AbstractCard> hand=phase.equals("upgrade")?logicalHand:player.hand.group;
        System.out.println("TRACE\t"+String.join("\t",scenario,Integer.toString(step),phase,
        ""+player.currentHealth,""+player.currentBlock,""+com.megacrit.cardcrawl.ui.panels.EnergyPanel.totalCount,
        cards(hand,false),cards(player.drawPile.group,true),cards(player.discardPile.group,false),cards(player.exhaustPile.group,false),
        ""+monster.currentHealth,""+monster.currentBlock,powers(player),powers(monster),""+AbstractDungeon.cardRandomRng.counter,""+AbstractDungeon.shuffleRng.counter,
        phase.equals("play")?"-":cards(options(),false)));
    }
    public static void main(String[] args)throws Exception {
        initialize();
        System.out.println("VERSION\t"+CardCrawlGame.VERSION_NUM);
        for(String line:java.nio.file.Files.readAllLines(java.nio.file.Path.of(args[0]))){
            if(line.isBlank()||line.startsWith("#"))continue;
            String[] row=line.split("\t",-1);
            switch(row[0]){
                case "case": reset(row);
                break;
                case "play":
                if(!phase().equals("play"))throw new IllegalStateException("unresolved choice");
                inUse=player.hand.group.get(Integer.parseInt(row[1]));
                logicalHand=new ArrayList<>(player.hand.group);
                logicalHand.remove(inUse);
                inUse.applyPowers();
                player.useCard(inUse,monster,com.megacrit.cardcrawl.ui.panels.EnergyPanel.totalCount);
                drain();
                ++step;
                break;
                case "choose":
                if(phase().equals("play"))throw new IllegalStateException("no choice");
                AbstractCard choice=options().get(Integer.parseInt(row[1]));
                if(phase().equals("topdeck_discard"))AbstractDungeon.gridSelectScreen.selectedCards.add(choice);
                else {
                    player.hand.group.remove(choice);
                    AbstractDungeon.handCardSelectScreen.selectedCards.addToTop(choice);
                    AbstractDungeon.handCardSelectScreen.wereCardsRetrieved=false;
                }
                AbstractDungeon.isScreenUp=false;
                AbstractDungeon.screen=AbstractDungeon.CurrentScreen.NONE;
                drain();
                ++step;
                break;
                default:throw new IllegalArgumentException("unknown directive "+row[0]);
            }
            snapshot();
        }
    }
}
