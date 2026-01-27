import { Composition, Folder } from 'remotion';
import { CurlRunnerPromo, CurlRunnerThumb } from './scenes/CurlRunnerPromo';

const fps = 30;
const width = 1920;
const height = 1080;
const durationInFrames = 450;

export const RemotionRoot = () => {
  return (
    <Folder name="curl-runner">
      <Composition
        id="curl-runner-promo"
        component={CurlRunnerPromo}
        width={width}
        height={height}
        fps={fps}
        durationInFrames={durationInFrames}
      />
      <Composition
        id="curl-runner-thumb"
        component={CurlRunnerThumb}
        width={width}
        height={height}
        fps={fps}
        durationInFrames={150}
      />
    </Folder>
  );
};
