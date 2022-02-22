<script lang="ts">

    import {onMount} from "svelte";
    import {Link} from "svelte-navigator";

    let images:string[] = []

    onMount(async()=>{
        images = await (await fetch("/api/images")).json()
    })

</script>

<div>
    pog
    <div class="images" >
        {#each images as img}
            <Link to="/image/{img}">
                <img class="pogimg" src="/api/images/{img}" />
            </Link>
        {/each}
    </div>
</div>

<style lang="css" >

    .images{
        display: flex;
        flex-wrap: wrap;
        max-height: 50%;
        background: #35373d;
        border-radius: 1em;
        overflow: hidden;
    }
    .pogimg {
        max-height: 30em;
        padding: .2em;
    }

</style>