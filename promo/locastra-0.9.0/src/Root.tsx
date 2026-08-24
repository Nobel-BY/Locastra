import React from 'react';
import {Composition} from 'remotion';
import {LocastraPromo} from './LocastraPromo';

export const Root: React.FC = () => (
  <>
    <Composition
      id="LocastraBilibili"
      component={LocastraPromo}
      width={1920}
      height={1080}
      fps={30}
      durationInFrames={900}
      defaultProps={{vertical: false}}
    />
    <Composition
      id="LocastraDouyin"
      component={LocastraPromo}
      width={1080}
      height={1920}
      fps={30}
      durationInFrames={900}
      defaultProps={{vertical: true}}
    />
  </>
);
