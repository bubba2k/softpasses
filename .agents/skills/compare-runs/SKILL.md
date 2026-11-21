# Skill goal

The goal of this skill is to compare the performance and output of two (OR MORE) arbitrary different dev versions of the software.

A version can be specified directly as a commit or branch or tag (preferable),
or more loosely by saying "what set of features that version had". It is then the task of the agent, to find out which commit that is.
 
# Expected input

The user needs to specify:
- The scene to be used for the test run
- Scene settings to be used: spp (default=1000), width and height (default to 1920x1080 if not given), the number of bounces (default to 10 if not given)
- The two version to be used.

# Process

Determine the commits/branches to compare.

For each: Perform a (release) run, storing its ouput to a new directory.
Each run produces a `renderstats.txt` in its output directory.
Use the contents of those to finalize the comparison.

# Constraints

Any uncommitted changes need to temporarily be stashed, and of course retrieved after the process!!
The working directory should be in IDENTICAL states before and after running this script, save for the new output rendered to image files.

IMPORTANT: BOTH of these runs MUST be made with identical scene and scene settings. ACCOUNT FOR THIS when stashing changes or checking out older commits or branches!!

# Expected output
- A brief description of the scene used for the comparison.
- The full render info output for BOTH runs.
- A summarizing comparison between the two: Compare rendertime and kRays/s from the render info.
- The image output of both runs inside new directories, with a short descriptive name prefixed by `comp_` inside `images/agentic/`.